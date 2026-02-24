#include <zinc/interrupts/interrupts.h>
#include <zinc/interrupts/idt.h>
#include <zinc/interrupts/pic.h>
#include <zinc/interrupts/keyboard.h>
#include <stdint.h>

static void cpuid(uint32_t leaf, uint32_t *a, uint32_t *b, uint32_t *c, uint32_t *d) {
    __asm__ volatile ("cpuid"
                      : "=a"(*a), "=b"(*b), "=c"(*c), "=d"(*d)
                      : "a"(leaf));
}

static uint64_t rdmsr(uint32_t msr) {
    uint32_t lo, hi;
    __asm__ volatile ("rdmsr" : "=a"(lo), "=d"(hi) : "c"(msr));
    return ((uint64_t)hi << 32) | lo;
}

static void wrmsr(uint32_t msr, uint64_t val) {
    uint32_t lo = (uint32_t)val;
    uint32_t hi = (uint32_t)(val >> 32);
    __asm__ volatile ("wrmsr" : : "c"(msr), "a"(lo), "d"(hi));
}

static void disable_apic_if_present(void) {
    uint32_t a, b, c, d;
    cpuid(1, &a, &b, &c, &d);
    if ((d & (1u << 5)) && (d & (1u << 9))) { 
        uint64_t apic = rdmsr(0x1B); 
        apic &= ~(1ULL << 11); 
        wrmsr(0x1B, apic);
    }
}

void interrupts_init(void) {
    idt_init();
    disable_apic_if_present();
    pic_set_imcr(0); 
    pic_remap(0x20, 0x28);
    for (uint8_t i = 0; i < 16; i++) {
        pic_set_mask(i);
    }
    pic_clear_mask(1); 
    keyboard_init();
}

void interrupts_enable(void) {
    __asm__ volatile ("sti");
}
