#pragma once

#include <stdint.h>

extern volatile uint64_t ticks;

void pit_init(uint32_t freq);
void timer_handler(void);
void sleep(uint64_t ms);
