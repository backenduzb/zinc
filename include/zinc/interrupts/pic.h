#pragma once
#include <stdint.h>

void pic_remap(uint8_t offset1, uint8_t offset2);
void pic_set_mask(uint8_t irq);
void pic_clear_mask(uint8_t irq);
void pic_eoi(uint8_t irq);
void pic_set_imcr(uint8_t mode);
