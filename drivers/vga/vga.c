#include <kernel/vga/colors.h>
#include <kernel/vga/cursor.h>
#include <kernel/vga/vga.h>
#include <stdint.h>

static volatile uint16_t *const VGA = (uint16_t *)0xB8000;

const int VGA_WIDTH = 80;
const int VGA_HEIGH = 25;

static int row = 0;
static int col = 0;
static uint8_t current_color = 0x07;

static inline uint16_t entry(char c, uint8_t color) {
  return (uint16_t)c | ((uint16_t)color << 8);
}

void vga_set_color(uint8_t fg, uint8_t bg) {
  current_color = vga_entry_color(fg, bg);
}

void vga_clear(void) {
  for (int i = 0; i < VGA_WIDTH *VGA_HEIGH; i++)
    VGA[i] = entry(' ', current_color);

  row = 0;
  col = 0;
  vga_set_cursor(row, col);
}

void putc(char c) {
  if (c == '\n') {
    col = 0;
    row++;
  } else {
    VGA[row * VGA_WIDTH + col] = entry(c, current_color);
    col++;
  }

  if (col >= VGA_WIDTH) {
    col = 0;
    row++;
  }

  if (row >=VGA_HEIGH) {
    for (int r = 1; r <VGA_HEIGH; r++) {
      for (int c = 0; c < VGA_WIDTH; c++) {
          VGA[(r - 1) * VGA_WIDTH + c] = VGA[r * VGA_WIDTH + c];
      }
    }
    uint16_t blank = entry(' ', current_color);
    for (int c = 0; c < VGA_WIDTH; c++) {
      VGA[24 * VGA_WIDTH + c] = blank;
    }

    row = 24;
  }

  vga_set_cursor(row, col);
}
void vga_write_soat(int r, int c, const char *sz) {
  if (r < 0 || r >= VGA_HEIGH || c < 0 || c >= VGA_WIDTH)
    return;
  for (int i = 0; sz[i] && (c + i) < VGA_WIDTH; i++) {
      VGA[r * VGA_WIDTH + c + i] = entry(sz[i], current_color);
  }
  vga_set_cursor(row, col);
}

void vga_write(const char *s) {
  for (int i = 0; s[i]; i++)
    putc(s[i]);
}

int vga_get_row(void) {
  return row;
}

int vga_get_col(void) {
  return col;
}

void vga_backspace(void){
    if (col > 0){
        col --;
    }
    else{
        row--;
        col=VGA_WIDTH;
    }
    VGA[row * VGA_WIDTH + col] = entry(' ', current_color);
    vga_set_cursor(row, col);
}

void vga_left(void) {
  if (col > 0) {
    col--;
  } else if (row > 0) {
    row--;
    col = 79;
  }
  vga_set_cursor(row, col);
}

void vga_right(void) {
  if (col > 0) {
    col++;
  } else if (row > 0) {
    row++;
    col = VGA_WIDTH -1;
    
  }
  vga_set_cursor(row, col);
}

void vga_bottom(void) {
  if (row > 0) {
    row++;
  } else
    return;

  vga_set_cursor(row, col);
}

void vga_top(void) {
  if (row > 0) {
    row--;
  } else
    return;

  vga_set_cursor(row, col);
}
