#pragma once
#include <stdint.h>

void idt_init(void);
void set_idt_gate(int n, uint32_t handler);
