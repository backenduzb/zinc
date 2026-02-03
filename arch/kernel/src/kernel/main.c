#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <kernel/pic.h>
#include <kernel/idt.h>
#include <kernel/io.h>

static inline void enable_interrupts(void){
    __asm__ volatile ("sti");
}

extern volatile uint8_t last_scancode;
extern volatile uint8_t kbd_has_data;

void kernel_main(void){
    vga_clear();
    vga_write("Zinc OS boot...\n");

    idt_init();
    pic_remap();

    outb(0x21, 0xFD); 
    outb(0xA1, 0xFF);

    __asm__ volatile("sti");
    vga_write("Keyboard ready. Press keys...\n");

    while(1){
        __asm__ volatile("hlt");

        if(kbd_has_data){
            kbd_has_data = 0;

            char hex[]="0123456789ABCDEF";
            char s[3];
            s[0]=hex[(last_scancode>>4)&0xF];
            s[1]=hex[last_scancode&0xF];
            s[2]=0;

            vga_write("Key ");
            vga_write(s);
            vga_write("\n");
        }
    }
}