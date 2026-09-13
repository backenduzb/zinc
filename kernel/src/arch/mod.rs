pub mod x86_64;

pub fn init() {
    x86_64::init();

    let cpu = crate::arch::x86_64::cpu::detect();
    
    crate::drivers::serial::print("CPU Vendor: ");
    
    for byte in cpu.vendor {
        crate::drivers::serial::putc(byte);
    }
    
    crate::drivers::serial::println("");
    
    crate::drivers::serial::print("CPU Family: ");
    crate::drivers::serial::print_u64(cpu.family as u64);
    crate::drivers::serial::println("");
    
    crate::drivers::serial::print("CPU Model: ");
    crate::drivers::serial::print_u64(cpu.model as u64);
    crate::drivers::serial::println("");
    
    crate::drivers::serial::print("CPU Stepping: ");
    crate::drivers::serial::print_u64(cpu.stepping as u64);
    crate::drivers::serial::println("");

    crate::arch::x86_64::cpu::print_features(&cpu);
}