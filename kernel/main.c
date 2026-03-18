#include <stdint.h>
#include <fb/drawer.h>
#include <fb/fb.h>
#include <idt.h>
#include <pic.h>
#include <timer/pit.h>

void kernel_main(uint64_t magic, uint64_t mbi_addr) {
    if (magic != MULTIBOOT2_BOOTLOADER_MAGIC) {
        while (1) { }
    }

    idt_init();
    pic_remap(0x20, 0x28);
    pic_set_mask(0, 0);
    pit_init(1000);
    __asm__ volatile("sti");
    
    init_framebuffer(mbi_addr);
    show_splash_screen();
    write_center("Welcome to ZINC !", 0x00FFFFFF);
    while (1) {
        draw_screen_border(0x00FFFFFF);
        sleep(500);
        draw_screen_border(0x0000FFF0);
        sleep(500);
        __asm__ volatile("hlt");
    }
}
