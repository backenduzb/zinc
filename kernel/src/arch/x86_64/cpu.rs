use core::arch::x86_64::{__cpuid, __cpuid_count};

pub struct CpuInfo {
    pub vendor: [u8; 12],

    pub family: u32,
    pub model: u32,
    pub stepping: u32,

    pub features_ecx: u32,
    pub features_edx: u32,

    pub initial_apic_id: u32,

    pub max_basic_leaf: u32,
    pub logical_processors: u32,

    pub apic_supported: bool,
    pub apic_enabled: bool,
    pub x2apic_enabled: bool,
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

    let apic_supported = (leaf1.edx & (1 << 9)) != 0;

    let fallback_logical_processors = {
        let count = (leaf1.ebx >> 16) & 0xFF;

        if count == 0 { 1 } else { count }
    };

    let (logical_processors, topology_supported) =
        detect_topology(max_basic_leaf, fallback_logical_processors);

    let (apic_enabled, x2apic_enabled) = detect_apic_state();

    CpuInfo {
        vendor,
        family,
        model,
        stepping,

        features_ecx: leaf1.ecx,
        features_edx: leaf1.edx,

        initial_apic_id,

        max_basic_leaf,
        logical_processors,

        apic_supported,
        apic_enabled,
        x2apic_enabled,
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

    let enabled = (apic_msr & (1 << 11)) != 0;
    let x2apic = (apic_msr & (1 << 10)) != 0;

    (enabled, x2apic)
}

impl CpuInfo {
    #[inline]
    pub fn has_ecx(&self, bit: u32) -> bool {
        (self.features_ecx & (1 << bit)) != 0
    }

    #[inline]
    pub fn has_edx(&self, bit: u32) -> bool {
        (self.features_edx & (1 << bit)) != 0
    }

    #[inline]
    pub fn apic_id(&self) -> u32 {
        self.initial_apic_id
    }
}

pub fn print_features(info: &CpuInfo) {
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
    crate::drivers::serial::println(if info.has_ecx(21) { "YES" } else { "NO" });

    crate::drivers::serial::print("  HTT: ");
    crate::drivers::serial::println(if info.has_edx(28) { "YES" } else { "NO" });

    crate::drivers::serial::print("  AVX: ");
    crate::drivers::serial::println(if info.has_ecx(28) { "YES" } else { "NO" });

    crate::drivers::serial::print("  RDRAND: ");
    crate::drivers::serial::println(if info.has_ecx(30) { "YES" } else { "NO" });

    crate::drivers::serial::println("");

    crate::drivers::serial::println("CPU Topology:");

    crate::drivers::serial::print("  Logical CPUs: ");
    crate::drivers::serial::print_u64(info.logical_processors as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  Initial APIC ID: ");
    crate::drivers::serial::print_u64(info.initial_apic_id as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  APIC supported: ");
    crate::drivers::serial::println(if info.apic_supported { "YES" } else { "NO" });

    crate::drivers::serial::print("  APIC enabled: ");
    crate::drivers::serial::println(if info.apic_enabled { "YES" } else { "NO" });

    crate::drivers::serial::print("  x2APIC enabled: ");
    crate::drivers::serial::println(if info.x2apic_enabled { "YES" } else { "NO" });
}
