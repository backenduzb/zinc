#include <kernel/vga.h>

void kernel_main(void){
    vga_clear();
    vga_write("Zinc OS\n");
    vga_write("VGA Succes working");

    for(;;) __asm__("hlt");
}
