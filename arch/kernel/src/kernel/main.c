#include <kernel/idt.h>
#include <kernel/io.h>
#include <kernel/keyboard/keyreader.h>
#include <kernel/pic.h>
#include <kernel/vga/colors.h>
#include <kernel/vga/vga.h>

static inline void enable_interrupts(void) { __asm__ volatile("sti"); }

extern volatile uint8_t last_scancode;
extern volatile uint8_t kbd_has_data;

void kernel_main(void) {
  vga_clear();
  vga_set_color(VGA_CYAN, VGA_BLACK);
  vga_write("                 Welcome to");
  vga_set_color(VGA_DARK_GREY, VGA_BLACK);
  vga_write(" Zinc");
  vga_set_color(VGA_CYAN, VGA_BLACK);
  vga_write(" OS!\n");
  vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);

  idt_init();
  pic_remap();

  outb(0x21, 0xFD);
  outb(0xA1, 0xFF);

  __asm__ volatile("sti");
  vga_write("Keyboard ready. Press keys...\n");

  while (1) {
    __asm__ volatile("hlt");
  }
}
