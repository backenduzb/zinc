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
    crate::log_smp!("CPUID.1:");
    crate::log_smp_raw!("  EAX: 0x");
    crate::log_smp_raw!("{:08X}", info.cpuid1_eax);
    crate::log_smp!("");
    crate::log_smp_raw!("  EBX: 0x");
    crate::log_smp_raw!("{:08X}", info.cpuid1_ebx);
    crate::log_smp!("");
    crate::log_smp_raw!("  ECX: 0x");
    crate::log_smp_raw!("{:08X}", info.features_ecx);
    crate::log_smp!("");
    crate::log_smp_raw!("  EDX: 0x");
    crate::log_smp_raw!("{:08X}", info.features_edx);
    crate::log_smp!("");
    crate::log_smp!("");

    crate::log_smp!("CPU Features:");

    crate::log_smp_raw!("  SSE: ");
    crate::log_smp!("{}", if info.has_edx(25) { "YES" } else { "NO" });

    crate::log_smp_raw!("  SSE2: ");
    crate::log_smp!("{}", if info.has_edx(26) { "YES" } else { "NO" });

    crate::log_smp_raw!("  SSE3: ");
    crate::log_smp!("{}", if info.has_ecx(0) { "YES" } else { "NO" });

    crate::log_smp_raw!("  SSSE3: ");
    crate::log_smp!("{}", if info.has_ecx(9) { "YES" } else { "NO" });

    crate::log_smp_raw!("  SSE4.1: ");
    crate::log_smp!("{}", if info.has_ecx(19) { "YES" } else { "NO" });

    crate::log_smp_raw!("  SSE4.2: ");
    crate::log_smp!("{}", if info.has_ecx(20) { "YES" } else { "NO" });

    crate::log_smp_raw!("  APIC: ");
    crate::log_smp!("{}", if info.apic_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  x2APIC: ");
    crate::log_smp!("{}", if info.x2apic_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  HTT: ");
    crate::log_smp!("{}", if info.has_edx(28) { "YES" } else { "NO" });

    crate::log_smp_raw!("  AVX: ");
    crate::log_smp!("{}", if info.avx_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  OSXSAVE: ");
    crate::log_smp!("{}", if info.osxsave_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  AVX usable (OS): ");
    crate::log_smp!("{}", if info.avx_usable { "YES" } else { "NO" });

    crate::log_smp_raw!("  RDRAND: ");
    crate::log_smp!("{}", if info.rdrand_supported { "YES" } else { "NO" });

    crate::log_smp!("");

    crate::log_smp!("CPU Topology:");

    crate::log_smp_raw!("  CPUID max logical (EBX[23:16]): ");
    crate::log_smp_raw!("{}", info.cpuid_max_logical as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("  CPUID logical CPUs: ");
    crate::log_smp_raw!("{}", info.logical_processors as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("  Initial APIC ID: ");
    crate::log_smp_raw!("{}", info.initial_apic_id as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("  APIC supported: ");
    crate::log_smp!("{}", if info.apic_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  LAPIC enabled: ");
    crate::log_smp!("{}", if info.apic_enabled { "YES" } else { "NO" });

    crate::log_smp_raw!("  x2APIC supported: ");
    crate::log_smp!("{}", if info.x2apic_supported { "YES" } else { "NO" });

    crate::log_smp_raw!("  x2APIC enabled: ");
    crate::log_smp!("{}", if info.x2apic_enabled { "YES" } else { "NO" });
}
