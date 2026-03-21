#include <idt.h>
#include <stdint.h>

extern void idt_load(struct idt_ptr *idtr);
extern void isr_timer(void);
extern void isr_keyboard(void);

static struct idt_entry idt[256];

static void idt_set_gate(uint8_t vector, void *isr) {
    uint64_t addr = (uint64_t)isr;

    idt[vector].offset_low = (uint16_t)(addr & 0xFFFF);
    idt[vector].selector = 0x08;
    idt[vector].ist = 0;
    idt[vector].type_attr = 0x8E;
    idt[vector].offset_mid = (uint16_t)((addr >> 16) & 0xFFFF);
    idt[vector].offset_high = (uint32_t)((addr >> 32) & 0xFFFFFFFF);
    idt[vector].zero = 0;
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

    idt_set_gate(32, isr_timer);
    idt_set_gate(33, isr_keyboard);

    struct idt_ptr idtr;
    idtr.limit = (uint16_t)(sizeof(idt) - 1);
    idtr.base = (uint64_t)&idt[0];

    idt_load(&idtr);
}
