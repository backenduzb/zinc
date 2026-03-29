#include <stdint.h>
#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <fb/fb.h>
#include <font/psf1.h>
#include <idt.h>
#include <input/keyboard.h>
#include <pic.h>
#include <ui/render.h>

extern const uint8_t _binary_font_psf_start[] __attribute__((weak));

void kernel_main(uint64_t magic, uint64_t mbi_addr) {
    if (magic != MULTIBOOT2_BOOTLOADER_MAGIC) {
        while (1) { }
    }
    
    idt_init();
    pic_remap(0x20, 0x28);
    pic_set_mask(0, 0);
    pic_set_mask(1, 1);
    pit_init(1000);

    init_framebuffer(mbi_addr);
    psf1_init(_binary_font_psf_start);
    keyboard_init();
    pic_set_mask(1, 0);
    
    
    __asm__ volatile("sti");
    show_splash_screen();
    draw_resolution();
    rtc_init();
    
    
    while (1) {
        update_time();
        render_ui();
        __asm__ volatile("hlt");
    }
}
