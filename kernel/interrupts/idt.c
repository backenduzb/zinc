#include <zinc/interrupts/idt.h>
#include <stdint.h>

#include <zinc/interrupts/keyboard.h>

static struct idt_entry idt[256];
static struct idt_ptr idt_reg;

static uint16_t read_cs(void) {
    uint16_t cs;
    __asm__ volatile ("mov %%cs, %0" : "=r"(cs));
    return cs;
}

static void idt_set_gate(uint8_t vec, void *isr) {
    uint64_t addr = (uint64_t)isr;
    idt[vec].offset_low = addr & 0xFFFF;
    idt[vec].selector = read_cs();
    idt[vec].ist = 0;
    idt[vec].type_attr = 0x8E;
    idt[vec].offset_mid = (addr >> 16) & 0xFFFF;
    idt[vec].offset_high = (addr >> 32) & 0xFFFFFFFF;
    idt[vec].zero = 0;
}

void idt_init(void) {
    for (int i = 0; i < 256; i++) {
        idt[i].offset_low = 0;
        idt[i].selector = 0;
        idt[i].ist = 0;
        idt[i].type_attr = 0;
        idt[i].offset_mid = 0;
        idt[i].offset_high = 0;
        idt[i].zero = 0;
    }

    idt_set_gate(33, irq1_handler);

    idt_reg.limit = sizeof(idt) - 1;
    idt_reg.base = (uint64_t)&idt[0];

    __asm__ volatile ("lidt %0" : : "m"(idt_reg));
}
