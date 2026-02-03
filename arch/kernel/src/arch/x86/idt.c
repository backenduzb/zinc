#include <stdint.h>

struct idt_entry {
  uint16_t base_lo;
  uint16_t sel;
  uint8_t always0;
  uint8_t flags;
  uint16_t base_hi;
} __attribute__((packed));

struct idt_ptr {
  uint16_t limit;
  uint32_t base;
} __attribute__((packed));

static struct idt_entry idt[256];
static struct idt_ptr idtp;

extern void isr33(void);
extern void isr_default(void);
extern void idt_load(uint32_t);
extern void isr_halt(void);

void set_idt_gate(int n, uint32_t handler) {
  idt[n].base_lo = handler & 0xFFFF;
  idt[n].sel = 0x10;
  idt[n].always0 = 0;
  idt[n].flags = 0x8E;
  idt[n].base_hi = (handler >> 16) & 0xFFFF;
}

void idt_init() {
  idtp.limit = sizeof(idt) - 1;
  idtp.base = (uint32_t)&idt;

  for (int i = 0; i < 32; i++) {
    set_idt_gate(i, (uint32_t)isr_halt);
  }
  for (int i = 32; i < 256; i++) {
    set_idt_gate(i, (uint32_t)isr_default);
  }
  
  set_idt_gate(33, (uint32_t)isr33);

  idt_load((uint32_t)&idtp);
}
