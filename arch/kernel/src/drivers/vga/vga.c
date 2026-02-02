#include <stdint.h>
#include <kernel/vga.h>

static volatile uint16_t* const VGA = (uint16_t*)0xB8000;

static int row = 0;
static int col = 0;

static uint16_t entry(char c) {
    return (uint16_t)c | (0x07 << 8);
}

void vga_clear(void) {
    for(int i=0;i<80*25;i++)
        VGA[i] = entry(' ');

    row = 0;
    col = 0;
} 

static void putc(char c) {
    if(c=='\n'){
        col=0;
        row++;
        return;
    }

    VGA[row*80+col] = entry(c);
    col++;

    if(col>=80){
        col=0;
        row++;
    }

    if(row>=25){
        row=0;  
    }
}

void vga_write(const char* s){
    for(int i=0;s[i];i++)
        putc(s[i]);
}