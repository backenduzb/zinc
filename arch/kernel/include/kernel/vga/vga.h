#ifndef KERNEL_VGA_H
#define KERNEL_VGA_H
#include <stdint.h>

void vga_clear(void);
void vga_write(const char* s);
void vga_set_color(uint8_t fg, uint8_t bg);
void putc(char c);
void vga_left(void);
void vga_top(void);
void vga_right(void);
void vga_bottom(void);
void vga_backspace(void);
void vga_write_soat(int row, int col, const char *sz);
int vga_get_row(void);
int vga_get_col(void);

#endif
