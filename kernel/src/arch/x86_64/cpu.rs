use core::arch::x86_64::{__cpuid, __cpuid_count};

pub struct CpuInfo {
    pub vendor: [u8; 12],

    pub family: u32,
    pub model: u32,
    pub stepping: u32,

    pub features_ecx: u32,
    pub features_edx: u32,

    pub cpuid1_eax: u32,
    pub cpuid1_ebx: u32,

    pub initial_apic_id: u32,

    pub max_basic_leaf: u32,

    pub cpuid_max_logical: u32,

    pub logical_processors: u32,

    pub apic_supported: bool,
    pub apic_enabled: bool,
    pub x2apic_supported: bool,
    pub x2apic_enabled: bool,

    pub avx_supported: bool,
    pub osxsave_supported: bool,
    pub avx_usable: bool,
    pub rdrand_supported: bool,
}

pub fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn detect() -> CpuInfo {
    let leaf0 = unsafe { __cpuid(0) };

    let max_basic_leaf = leaf0.eax;

    let mut vendor = [0u8; 12];

    vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());

    let leaf1 = unsafe { __cpuid(1) };

    let stepping = leaf1.eax & 0xF;
    let base_model = (leaf1.eax >> 4) & 0xF;
    let base_family = (leaf1.eax >> 8) & 0xF;

    let extended_model = (leaf1.eax >> 16) & 0xF;
    let extended_family = (leaf1.eax >> 20) & 0xFF;

    let initial_apic_id = (leaf1.ebx >> 24) & 0xFF;

    let family = if base_family == 0xF {
        base_family + extended_family
    } else {
        base_family
    };

    let model = if base_family == 0x6 || base_family == 0xF {
        base_model + (extended_model << 4)
    } else {
        base_model
    };

    let apic_supported = (leaf1.edx & (1u32 << 9)) != 0;

    let cpuid_max_logical = {
        let count = (leaf1.ebx >> 16) & 0xFF;

        if count == 0 { 1 } else { count }
    };

    let (logical_processors, _topology_from_leaf_b) =
        detect_topology(max_basic_leaf, cpuid_max_logical);

    let (apic_enabled, x2apic_enabled) = detect_apic_state();

    let x2apic_supported = (leaf1.ecx & (1u32 << 21)) != 0;
    let avx_supported = (leaf1.ecx & (1u32 << 28)) != 0;
    let osxsave_supported = (leaf1.ecx & (1u32 << 27)) != 0;
    let rdrand_supported = (leaf1.ecx & (1u32 << 30)) != 0;
    let avx_usable = avx_supported && osxsave_supported && xcr0_avx_enabled();

    CpuInfo {
        vendor,
        family,
        model,
        stepping,

        features_ecx: leaf1.ecx,
        features_edx: leaf1.edx,

        cpuid1_eax: leaf1.eax,
        cpuid1_ebx: leaf1.ebx,

        initial_apic_id,

        max_basic_leaf,
        cpuid_max_logical,
        logical_processors,

        apic_supported,
        apic_enabled,
        x2apic_supported,
        x2apic_enabled,

        avx_supported,
        osxsave_supported,
        avx_usable,
        rdrand_supported,
    }
}

fn detect_topology(max_basic_leaf: u32, fallback_logical_processors: u32) -> (u32, bool) {
    if max_basic_leaf >= 0xB {
        let leaf_b = unsafe { __cpuid_count(0xB, 1) };

        let logical_processors = (leaf_b.ebx & 0xFFFF) as u32;

        if logical_processors != 0 {
            return (logical_processors, true);
        }
    }

    (fallback_logical_processors, false)
}

fn detect_apic_state() -> (bool, bool) {
    let apic_msr = unsafe { crate::arch::x86_64::io::rdmsr(0x1B) };

    let enabled = (apic_msr & (1u64 << 11)) != 0;
    let x2apic = (apic_msr & (1u64 << 10)) != 0;

    (enabled, x2apic)
}

fn xcr0_avx_enabled() -> bool {
    let cr4: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nomem, nostack, preserves_flags),
        );
    }
    if (cr4 & (1u64 << 18)) == 0 {
        return false;
    }
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "xgetbv",
            in("ecx") 0u32,
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags),
        );
    }
    let xcr0 = ((hi as u64) << 32) | (lo as u64);
    (xcr0 & 0x6) == 0x6
}

impl CpuInfo {
    #[inline]
    pub fn has_ecx(&self, bit: u32) -> bool {
        (self.features_ecx & (1u32 << bit)) != 0
    }

    #[inline]
    pub fn has_edx(&self, bit: u32) -> bool {
        (self.features_edx & (1u32 << bit)) != 0
    }

    #[inline]
    pub fn apic_id(&self) -> u32 {
        self.initial_apic_id
    }
}

pub fn print_features(info: &CpuInfo) {
    crate::drivers::serial::println("CPUID.1:");
    crate::drivers::serial::print("  EAX: 0x");
    crate::drivers::serial::print_hex_u32(info.cpuid1_eax);
    crate::drivers::serial::println("");
    crate::drivers::serial::print("  EBX: 0x");
    crate::drivers::serial::print_hex_u32(info.cpuid1_ebx);
    crate::drivers::serial::println("");
    crate::drivers::serial::print("  ECX: 0x");
    crate::drivers::serial::print_hex_u32(info.features_ecx);
    crate::drivers::serial::println("");
    crate::drivers::serial::print("  EDX: 0x");
    crate::drivers::serial::print_hex_u32(info.features_edx);
    crate::drivers::serial::println("");
    crate::drivers::serial::println("");

    crate::drivers::serial::println("CPU Features:");

    crate::drivers::serial::print("  SSE: ");
    crate::drivers::serial::println(if info.has_edx(25) { "YES" } else { "NO" });

    crate::drivers::serial::print("  SSE2: ");
    crate::drivers::serial::println(if info.has_edx(26) { "YES" } else { "NO" });

    crate::drivers::serial::print("  SSE3: ");
    crate::drivers::serial::println(if info.has_ecx(0) { "YES" } else { "NO" });

    crate::drivers::serial::print("  SSSE3: ");
    crate::drivers::serial::println(if info.has_ecx(9) { "YES" } else { "NO" });

    crate::drivers::serial::print("  SSE4.1: ");
    crate::drivers::serial::println(if info.has_ecx(19) { "YES" } else { "NO" });

    crate::drivers::serial::print("  SSE4.2: ");
    crate::drivers::serial::println(if info.has_ecx(20) { "YES" } else { "NO" });

    crate::drivers::serial::print("  APIC: ");
    crate::drivers::serial::println(if info.apic_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  x2APIC: ");
    crate::drivers::serial::println(if info.x2apic_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  HTT: ");
    crate::drivers::serial::println(if info.has_edx(28) { "YES" } else { "NO" });

    crate::drivers::serial::print("  AVX: ");
    crate::drivers::serial::println(if info.avx_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  OSXSAVE: ");
    crate::drivers::serial::println(if info.osxsave_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  AVX usable (OS): ");
    crate::drivers::serial::println(if info.avx_usable { "YES" } else { "NO" });

    crate::drivers::serial::print("  RDRAND: ");
    crate::drivers::serial::println(if info.rdrand_supported { "YES" } else { "NO" });

    crate::drivers::serial::println("");

    crate::drivers::serial::println("CPU Topology:");

    crate::drivers::serial::print("  CPUID max logical (EBX[23:16]): ");
    crate::drivers::serial::print_u64(info.cpuid_max_logical as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  CPUID logical CPUs: ");
    crate::drivers::serial::print_u64(info.logical_processors as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  Initial APIC ID: ");
    crate::drivers::serial::print_u64(info.initial_apic_id as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  APIC supported: ");
    crate::drivers::serial::println(if info.apic_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  LAPIC enabled: ");
    crate::drivers::serial::println(if info.apic_enabled { "YES" } else { "NO" });

    crate::drivers::serial::print("  x2APIC supported: ");
    crate::drivers::serial::println(if info.x2apic_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  x2APIC enabled: ");
    crate::drivers::serial::println(if info.x2apic_enabled { "YES" } else { "NO" });
}
