pub mod x86_64;

pub fn init() {
    x86_64::init();

    let cpu = crate::arch::x86_64::cpu::detect();
    
    crate::log_smp_raw!("CPU Vendor: ");
    
    for byte in cpu.vendor {
        crate::log_smp_raw!("{}", (byte) as char);
    }
    
    crate::log_smp!("");
    
    crate::log_smp_raw!("CPU Family: ");
    crate::log_smp_raw!("{}", cpu.family as u64);
    crate::log_smp!("");
    
    crate::log_smp_raw!("CPU Model: ");
    crate::log_smp_raw!("{}", cpu.model as u64);
    crate::log_smp!("");
    
    crate::log_smp_raw!("CPU Stepping: ");
    crate::log_smp_raw!("{}", cpu.stepping as u64);
    crate::log_smp!("");

    crate::arch::x86_64::cpu::print_features(&cpu);
}