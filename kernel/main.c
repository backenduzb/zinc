#include <stdint.h>
#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <fb/fb.h>
#include <font/psf1.h>
#include <idt.h>
#include <input/keyboard.h>
#include <acpi.h>
#include <pic.h>
#include <utils/string.h>

extern const uint8_t _binary_font_psf_start[] __attribute__((weak));

static void format_temp(char *out, int temp) {
    if (temp < 0) {
        out[0] = 'N';
        out[1] = '/';
        out[2] = 'A';
        out[3] = 0;
        return;
    }

    uitoa((uint32_t)temp, out);
}

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

    acpit_init();

    __asm__ volatile("sti");
    show_splash_screen();
    draw_resolution();

    while (1) {
        int temp = acpi_get_cpu_temp();
        char temp_buf[12];
        format_temp(temp_buf, temp);
        write_center(temp_buf, 0x00FFFFFF);
        sleep(1000);
        write_center(temp_buf, 0x00000000);
        __asm__ volatile("hlt");
    }
}
