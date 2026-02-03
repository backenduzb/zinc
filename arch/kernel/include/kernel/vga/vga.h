#ifndef KERNEL_VGA_H
#define KERNEL_VGA_H
#include <stdint.h>

void vga_clear(void);
void vga_write(const char* s);
void vga_set_color(uint8_t fg, uint8_t bg);

#endif
