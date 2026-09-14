use core::ptr::read_unaligned;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtHeader {
    header: SdtHeader,
    local_apic_address: u32,
    flags: u32,
}

static mut LAPIC_PHYS_ADDR: u64 = 0;

/// MADT dan topilgan LAPIC physical base. 0 bo'lsa hali topilmagan.
pub fn lapic_phys() -> u64 {
    unsafe { LAPIC_PHYS_ADDR }
}

fn set_lapic_phys(addr: u64) {
    unsafe {
        LAPIC_PHYS_ADDR = addr;
    }
}

fn phys_to_virt(p: u64) -> u64 {
    if p >= 0xffff_8000_0000_0000 {
        return p;
    }
    if let Some(v) = crate::boot::limine::physical_to_virtual(p) {
        v
    } else {
        p
    }
}

fn checksum_ok(ptr: u64, len: usize) -> bool {
    let mut sum: u8 = 0;
    for i in 0..len {
        let b = unsafe { core::ptr::read((ptr + i as u64) as *const u8) };
        sum = sum.wrapping_add(b);
    }
    sum == 0
}

fn rsdp_checksum_ok(rsdp_virt: u64, revision: u8, length: u32) -> bool {
    let len = if revision >= 2 && length >= 20 && length <= 4096 {
        length as usize
    } else {
        20
    };
    checksum_ok(rsdp_virt, len)
}

fn sdt_checksum_ok(sdt_virt: u64) -> bool {
    let header = unsafe { read_unaligned(sdt_virt as *const SdtHeader) };
    let len = unsafe { core::ptr::addr_of!(header.length).read_unaligned() } as usize;
    if len < core::mem::size_of::<SdtHeader>() || len > 1024 * 1024 {
        return false;
    }
    checksum_ok(sdt_virt, len)
}

pub fn init() {
    let rsdp_phys = match crate::boot::limine::rsdp_address() {
        Some(addr) => addr,
        None => {
            crate::drivers::serial::println("SMP: RSDP not found");
            return;
        }
    };

    crate::drivers::serial::println("SMP: RSDP found");

    crate::drivers::serial::print("SMP: RSDP address: 0x");
    crate::drivers::serial::print_hex_u64(rsdp_phys);
    crate::drivers::serial::println("");

    let rsdp_virt = phys_to_virt(rsdp_phys);
    let rsdp = unsafe { read_unaligned(rsdp_virt as *const Rsdp) };

    if unsafe { core::ptr::addr_of!(rsdp.signature).read_unaligned() } != *b"RSD PTR " {
        crate::drivers::serial::println("SMP: Invalid RSDP signature");
        return;
    }

    crate::drivers::serial::println("SMP: RSDP signature OK");

    let revision = unsafe { core::ptr::addr_of!(rsdp.revision).read_unaligned() };
    let length = unsafe { core::ptr::addr_of!(rsdp.length).read_unaligned() };
    let xsdt_address = unsafe { core::ptr::addr_of!(rsdp.xsdt_address).read_unaligned() };
    let rsdt_address = unsafe { core::ptr::addr_of!(rsdp.rsdt_address).read_unaligned() };

    crate::drivers::serial::print("SMP: RSDP revision: ");
    crate::drivers::serial::print_u64(revision as u64);
    crate::drivers::serial::println("");

    if revision >= 2 {
        crate::drivers::serial::print("SMP: RSDP length: ");
        crate::drivers::serial::print_u64(length as u64);
        crate::drivers::serial::println("");

        crate::drivers::serial::print("SMP: XSDT address: 0x");
        crate::drivers::serial::print_hex_u64(xsdt_address);
        crate::drivers::serial::println("");

        if xsdt_address == 0 {
            crate::drivers::serial::println("SMP: XSDT address is null");
            return;
        }

        let xsdt_virt = phys_to_virt(xsdt_address);
        let ok = sdt_checksum_ok(xsdt_virt);
        crate::drivers::serial::print("SMP: XSDT checksum: ");
        if ok {
            crate::drivers::serial::println("OK");
        } else {
            crate::drivers::serial::println("BAD");
        }

        let xsdt_header = unsafe { read_unaligned(xsdt_virt as *const SdtHeader) };
        let xsdt_len = unsafe { core::ptr::addr_of!(xsdt_header.length).read_unaligned() } as usize;

        if xsdt_len < core::mem::size_of::<SdtHeader>() {
            crate::drivers::serial::println("SMP: XSDT length invalid");
            return;
        }

        let entries = (xsdt_len - core::mem::size_of::<SdtHeader>()) / 8;
        crate::drivers::serial::print("SMP: XSDT entries: ");
        crate::drivers::serial::print_u64(entries as u64);
        crate::drivers::serial::println("");

        let mut madt_phys: u64 = 0;
        for i in 0..entries {
            let entry_ptr = xsdt_virt + core::mem::size_of::<SdtHeader>() as u64 + (i as u64) * 8;
            let table_phys = unsafe { read_unaligned(entry_ptr as *const u64) };
            if table_phys == 0 {
                continue;
            }
            let table_virt = phys_to_virt(table_phys);
            let h = unsafe { read_unaligned(table_virt as *const SdtHeader) };
            let sig = unsafe { core::ptr::addr_of!(h.signature).read_unaligned() };
            if &sig == b"APIC" {
                madt_phys = table_phys;
                break;
            }
        }

        if madt_phys == 0 {
            crate::drivers::serial::println("SMP: MADT not found");
            return;
        }

        crate::drivers::serial::println("SMP: MADT found");

        let madt_virt = phys_to_virt(madt_phys);
        let madt = unsafe { read_unaligned(madt_virt as *const MadtHeader) };
        let lapic_addr = unsafe { core::ptr::addr_of!(madt.local_apic_address).read_unaligned() };
        crate::drivers::serial::print("SMP: Local APIC address: 0x");
        crate::drivers::serial::print_hex_u64(lapic_addr as u64);
        crate::drivers::serial::println("");

        set_lapic_phys(lapic_addr as u64);

        let madt_len = unsafe { core::ptr::addr_of!(madt.header.length).read_unaligned() } as usize;
        let mut offset = core::mem::size_of::<MadtHeader>();
        let mut cpu_index: u64 = 0;

        while offset + 2 <= madt_len {
            let entry_ptr = madt_virt + offset as u64;
            let entry_type = unsafe { core::ptr::read(entry_ptr as *const u8) };
            let entry_len = unsafe { core::ptr::read((entry_ptr + 1) as *const u8) } as usize;
            if entry_len < 2 || offset + entry_len > madt_len {
                break;
            }
            if entry_type == 0 && entry_len == 8 {
                let apic_id = unsafe { core::ptr::read((entry_ptr + 3) as *const u8) };
                let flags = unsafe { read_unaligned((entry_ptr + 4) as *const u32) };
                let enabled = (flags & 1) != 0 || (flags & 2) != 0;
                if enabled {
                    crate::drivers::serial::print("CPU ");
                    crate::drivers::serial::print_u64(cpu_index);
                    crate::drivers::serial::print(": APIC ID = ");
                    crate::drivers::serial::print_u64(apic_id as u64);
                    crate::drivers::serial::println("");
                    cpu_index += 1;
                } else {
                    crate::drivers::serial::print("CPU ");
                    crate::drivers::serial::print_u64(cpu_index);
                    crate::drivers::serial::print(": APIC ID = ");
                    crate::drivers::serial::print_u64(apic_id as u64);
                    crate::drivers::serial::print(" (disabled)");
                    crate::drivers::serial::println("");
                }
            } else if entry_type == 9 && entry_len == 16 {
                let apic_id = unsafe { read_unaligned((entry_ptr + 4) as *const u32) };
                let flags = unsafe { read_unaligned((entry_ptr + 8) as *const u32) };
                let enabled = (flags & 1) != 0;
                if enabled {
                    crate::drivers::serial::print("CPU ");
                    crate::drivers::serial::print_u64(cpu_index);
                    crate::drivers::serial::print(": APIC ID = ");
                    crate::drivers::serial::print_u64(apic_id as u64);
                    crate::drivers::serial::println("");
                    cpu_index += 1;
                }
            }
            if entry_len == 0 {
                break;
            }
            offset += entry_len;
        }

        if cpu_index == 0 {
            crate::drivers::serial::println("SMP: No enabled CPUs in MADT");
        }
    } else {
        crate::drivers::serial::println("SMP: ACPI 1.0 RSDT detected");
    
        crate::drivers::serial::print("SMP: RSDT address: 0x");
        crate::drivers::serial::print_hex_u64(rsdt_address as u64);
        crate::drivers::serial::println("");
    
        if rsdt_address == 0 {
            crate::drivers::serial::println("SMP: RSDT address is null");
            return;
        }
    
        let rsdt_virt = phys_to_virt(rsdt_address as u64);
    
        // RSDT checksum
        let ok = sdt_checksum_ok(rsdt_virt);
    
        crate::drivers::serial::print("SMP: RSDT checksum: ");
        if ok {
            crate::drivers::serial::println("OK");
        } else {
            crate::drivers::serial::println("BAD");
            return;
        }
    
        // RSDT header
        let rsdt_header =
            unsafe { read_unaligned(rsdt_virt as *const SdtHeader) };
    
        let rsdt_len =
            unsafe {
                core::ptr::addr_of!(rsdt_header.length).read_unaligned()
            } as usize;
    
        if rsdt_len < core::mem::size_of::<SdtHeader>() {
            crate::drivers::serial::println("SMP: RSDT length invalid");
            return;
        }
    
        let entries =
            (rsdt_len - core::mem::size_of::<SdtHeader>()) / 4;
    
        crate::drivers::serial::print("SMP: RSDT entries: ");
        crate::drivers::serial::print_u64(entries as u64);
        crate::drivers::serial::println("");
    
        // Search MADT (APIC)
        let mut madt_phys: u64 = 0;
    
        for i in 0..entries {
            let entry_ptr =
                rsdt_virt
                    + core::mem::size_of::<SdtHeader>() as u64
                    + (i as u64) * 4;
    
            let table_phys =
                unsafe {
                    read_unaligned(entry_ptr as *const u32)
                } as u64;
    
            if table_phys == 0 {
                continue;
            }
    
            let table_virt = phys_to_virt(table_phys);
    
            let table_header =
                unsafe {
                    read_unaligned(table_virt as *const SdtHeader)
                };
    
            let signature =
                unsafe {
                    core::ptr::addr_of!(table_header.signature)
                        .read_unaligned()
                };
    
            crate::drivers::serial::print("SMP: RSDT entry ");
            crate::drivers::serial::print_u64(i as u64);
            crate::drivers::serial::print(": ");
    
            for byte in signature {
                crate::drivers::serial::putc(byte as u8);
            }
    
            crate::drivers::serial::println("");
    
            if &signature == b"APIC" {
                madt_phys = table_phys;
                break;
            }
        }
    
        if madt_phys == 0 {
            crate::drivers::serial::println("SMP: MADT not found");
            return;
        }
    
        crate::drivers::serial::println("SMP: MADT found");
    
        let madt_virt = phys_to_virt(madt_phys);
    
        let madt =
            unsafe {
                read_unaligned(madt_virt as *const MadtHeader)
            };
    
        let lapic_addr =
            unsafe {
                core::ptr::addr_of!(madt.local_apic_address)
                    .read_unaligned()
            };

        set_lapic_phys(lapic_addr as u64);
    
        crate::drivers::serial::print(
            "SMP: Local APIC address: 0x"
        );
        crate::drivers::serial::print_hex_u64(lapic_addr as u64);
        crate::drivers::serial::println("");
    
        let madt_len =
            unsafe {
                core::ptr::addr_of!(madt.header.length)
                    .read_unaligned()
            } as usize;
    
        if madt_len < core::mem::size_of::<MadtHeader>() {
            crate::drivers::serial::println("SMP: MADT length invalid");
            return;
        }
    
        let mut offset = core::mem::size_of::<MadtHeader>();
        let mut cpu_index: u64 = 0;
    
        while offset + 2 <= madt_len {
            let entry_ptr = madt_virt + offset as u64;
    
            let entry_type =
                unsafe {
                    core::ptr::read(entry_ptr as *const u8)
                };
    
            let entry_len =
                unsafe {
                    core::ptr::read((entry_ptr + 1) as *const u8)
                } as usize;
    
            if entry_len < 2 || offset + entry_len > madt_len {
                break;
            }
    
            // Processor Local APIC
            if entry_type == 0 && entry_len == 8 {
                let processor_id =
                    unsafe {
                        core::ptr::read(
                            (entry_ptr + 2) as *const u8
                        )
                    };
    
                let apic_id =
                    unsafe {
                        core::ptr::read(
                            (entry_ptr + 3) as *const u8
                        )
                    };
    
                let flags =
                    unsafe {
                        read_unaligned(
                            (entry_ptr + 4) as *const u32
                        )
                    };
    
                let enabled =
                    (flags & 1) != 0 || (flags & 2) != 0;
    
                crate::drivers::serial::print("CPU ");
                crate::drivers::serial::print_u64(cpu_index);
                crate::drivers::serial::print(": Processor ID = ");
                crate::drivers::serial::print_u64(processor_id as u64);
    
                crate::drivers::serial::print(", APIC ID = ");
                crate::drivers::serial::print_u64(apic_id as u64);
    
                if enabled {
                    crate::drivers::serial::print("CPU ");
                    crate::drivers::serial::print_u64(cpu_index);
                    crate::drivers::serial::print(": Processor ID = ");
                    crate::drivers::serial::print_u64(processor_id as u64);
                    crate::drivers::serial::print(", APIC ID = ");
                    crate::drivers::serial::print_u64(apic_id as u64);
                    crate::drivers::serial::println(" [enabled]");
                
                    super::register_cpu(
                        processor_id,
                        apic_id as u32,
                        true,
                    );
                
                    cpu_index += 1;
                } else {
                    crate::drivers::serial::println(" [disabled]");
                }
            }
    
            // Processor Local x2APIC
            else if entry_type == 9 && entry_len == 16 {
                let processor_uid =
                    unsafe {
                        read_unaligned(
                            (entry_ptr + 4) as *const u32
                        )
                    };
    
                let flags =
                    unsafe {
                        read_unaligned(
                            (entry_ptr + 8) as *const u32
                        )
                    };
    
                let apic_id =
                    unsafe {
                        read_unaligned(
                            (entry_ptr + 12) as *const u32
                        )
                    };
    
                if (flags & 1) != 0 {
                    crate::drivers::serial::print(
                        "CPU x2APIC: UID = "
                    );
                    crate::drivers::serial::print_u64(
                        processor_uid as u64
                    );
    
                    crate::drivers::serial::print(
                        ", APIC ID = "
                    );
                    crate::drivers::serial::print_u64(
                        apic_id as u64
                    );
    
                    crate::drivers::serial::println(
                        " [enabled]"
                    );
    
                    cpu_index += 1;
                }
            }
    
            offset += entry_len;
        }
    
        if cpu_index == 0 {
            crate::drivers::serial::println(
                "SMP: No enabled CPUs in MADT"
            );
        } else {
            crate::drivers::serial::print(
                "SMP: Enabled CPUs: "
            );
            crate::drivers::serial::print_u64(cpu_index);
            crate::drivers::serial::println("");
        }
    }

    let _ = rsdp_checksum_ok(rsdp_virt, revision, length);

    if !rsdp_checksum_ok(rsdp_virt, revision, length) {
        crate::drivers::serial::println(
            "SMP: RSDP checksum: BAD"
        );
        return;
    }
    
    crate::drivers::serial::println(
        "SMP: RSDP checksum: OK"
    );
}
