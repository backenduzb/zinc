#include <stdint.h>
#include <fb/fb.h>

void kernel_main(uint64_t magic, uint64_t mbi_addr) {
    init_framebuffer(mbi_addr);
    draw_line();

    while (1) { }
}
