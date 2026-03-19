#include <stdint.h>
#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <fb/fb.h>
#include <font/psf1.h>
#include <idt.h>
#include <pic.h>

extern const uint8_t _binary_font_psf_start[] __attribute__((weak));

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
    psf1_init(_binary_font_psf_start);
    
    show_splash_screen();
    draw_resolution();
    write_center_with_duration("Welcome to ZINC OS !", 0x00FFFFFF, 100);
    sleep(1000);
    write_center_with_duration("Welcome to ZINC OS !", 0x00000000, 100);
    sleep(1000);
    // write_center_with_duration("On this time we have only clock XD", 0x00FFFFFF, 100);
    // sleep(1000);
    // write_center_with_duration("On this time we have only clock XD", 0x00000000, 100);
    
    // write_center_with_duration("Others news coming soon", 0x00FFFFFF, 100);
    // sleep(1000);
    // write_center_with_duration("Others news coming soon", 0x00000000, 100);
    
    draw_background();
    while (1) {
        char time_b[9];
        get_time(time_b);
        write_center(time_b, 0x00FFFFFF);
        sleep(1000);
        write_center(time_b, 0x00000000);
        __asm__ volatile("hlt");
    }
}
