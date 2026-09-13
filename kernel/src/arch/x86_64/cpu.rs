use core::arch::x86_64::__cpuid;

pub struct CpuInfo {
    pub vendor: [u8; 12],
    pub family: u32,
    pub model: u32,
    pub stepping: u32,

    pub features_ecx: u32,
    pub features_edx: u32,
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

    let mut vendor = [0u8; 12];

    vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());

    let leaf1 = unsafe { __cpuid(1) };

    let stepping = leaf1.eax & 0xF;
    let base_model = (leaf1.eax >> 4) &0xF;
    let base_family = (leaf1.eax >> 8) &0xF;

    let extended_model = (leaf1.eax >> 16) & 0xF;
    let extended_family = (leaf1.eax >> 20) & 0xFF;

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

    CpuInfo {
        vendor,
        family,
        model,
        stepping,
        features_ecx: leaf1.ecx,
        features_edx: leaf1.edx,
    }
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
    crate::drivers::serial::println(if info.has_edx(9) { "YES" } else { "NO" });

    crate::drivers::serial::print("  x2APIC: ");
    crate::drivers::serial::println(if info.has_ecx(21) { "YES" } else { "NO" });

    crate::drivers::serial::print("  HTT: ");
    crate::drivers::serial::println(if info.has_edx(28) { "YES" } else { "NO" });

    crate::drivers::serial::print("  AVX: ");
    crate::drivers::serial::println(if info.has_ecx(28) { "YES" } else { "NO" });

    crate::drivers::serial::print("  RDRAND: ");
    crate::drivers::serial::println(if info.has_ecx(30) { "YES" } else { "NO" });
}