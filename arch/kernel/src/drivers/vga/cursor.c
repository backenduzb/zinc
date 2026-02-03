#include <kernel/io.h>
#include <stdint.h>

void vga_set_cursor(int row, int col) {
  uint16_t position = row * 80 + col;

  outb(0x3D4, 0x0F);
  outb(0x3D5, (uint8_t)(position & 0xFF));

  outb(0x3D4, 0x0E);
  outb(0x3D5, (uint8_t)((position >> 8) & 0xFF));
}
