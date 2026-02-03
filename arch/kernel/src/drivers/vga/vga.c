#include <stdint.h>
#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <kernel/vga/cursor.h>

static volatile uint16_t* const VGA = (uint16_t*)0xB8000;

static int row = 0;
static int col = 0;
static uint8_t current_color = 0x07;

static inline uint16_t entry(char c, uint8_t color) {
    return (uint16_t)c | ((uint16_t)color << 8);
}

void vga_set_color(uint8_t fg, uint8_t bg) {
    current_color = vga_entry_color(fg, bg);
}

void vga_clear(void) {
    for(int i=0;i<80*25;i++)
        VGA[i] = entry(' ', current_color);

    row = 0;
    col = 0;
} 

void putc(char c){
    if(c=='\n'){
        col = 0;
        row++;
    } else {
        VGA[row*80+col] = entry(c, current_color);
        col++;
    }
    
    if(col >= 80){
        col = 0;
        row++;
    }

    if(row >= 25){
        for (int r = 1; r < 25; r++) {
            for (int c = 0; c < 80; c++) {
                VGA[(r - 1) * 80 + c] = VGA[r * 80 + c];
            }
        }
        uint16_t blank = entry(' ', current_color);
        for (int c = 0; c < 80; c++) {
            VGA[24 * 80 + c] = blank;
        }
        
        row = 24;
    }

    vga_set_cursor(row, col);
}


void vga_write(const char* s){
    for(int i=0;s[i];i++)
        putc(s[i]);
}
