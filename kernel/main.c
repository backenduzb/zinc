#include <stdint.h>
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

    //sleep(3000);
    //draw_line();
    //sleep(5000);
    //draw_line_white();

    while (1) {
        sleep(3000);
        draw_line();
        sleep(3000);
        draw_line_white();
    }
}
