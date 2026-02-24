#pragma once
#include <stdint.h>

struct interrupt_frame;

void keyboard_init(void);
void irq1_handler(struct interrupt_frame *frame);
void keyboard_poll(void);
uint64_t keyboard_irq_count(void);
