#include <power.h>
#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>
#include <ui/layout.h>
#include <timer/pit.h>
#include <time/time.h>
#define CURSOR_HEIGHT 3
#define CURSOR_WIDTH 8

uint32_t col = 15;
uint32_t row = 38;
char *term_pointer = "$root%zinc > ";
int GLYPH_HEIGHT = 16;
int opened = 0;

char key_counter[256];
int key_index = 0;

uint32_t cursor_backup[CURSOR_HEIGHT * CURSOR_WIDTH];
uint32_t row_prev = 0;
uint32_t col_prev = 0;
uint64_t cursor_last_toggle = 0;
uint8_t cursor_on = 1;

void cursor_refresh() {
    if (ticks - cursor_last_toggle >= 700 && opened) {
        cursor_last_toggle = ticks;
        cursor_on = !cursor_on;

        uint32_t pitch = width;

        if (cursor_on) {
            for (uint32_t h = 0; h < CURSOR_HEIGHT; h++)
                for (uint32_t w = 0; w < CURSOR_WIDTH; w++)
                    framebuffer[(row  + 10 + h) * pitch + (col + w)] = 0x00FFFFFF;
        } else {
            for (uint32_t h = 0; h < CURSOR_HEIGHT; h++)
                for (uint32_t w = 0; w < CURSOR_WIDTH; w++)
                    framebuffer[(row + 10 + h) * pitch + (col + w)] = cursor_backup[h * CURSOR_WIDTH + w];
        }
    }
}

void cursor_update(uint32_t row, uint32_t col, uint32_t color) {
    uint32_t pitch = width;

    for (uint32_t h = 0; h < CURSOR_HEIGHT; h++) {
        for (uint32_t w = 0; w < CURSOR_WIDTH; w++) {
            framebuffer[(row_prev + h) * pitch + (col_prev + w)] = cursor_backup[h * CURSOR_WIDTH + w];
        }
    }
    for (uint32_t h = 0; h < CURSOR_HEIGHT; h++) {
        for (uint32_t w = 0; w < CURSOR_WIDTH; w++) {
            cursor_backup[h * CURSOR_WIDTH + w] = framebuffer[(row + h) * pitch + (col + w)];
            framebuffer[(row + h) * pitch + (col + w)] = color;
        }
    }

    row_prev = row;
    col_prev = col;
}

static void clear_term() {
    for (uint32_t r = 30; r < (height); r++) {
        for (uint32_t c = 15; c < (width - 15); c++) {
            framebuffer[r * width + c] = 0x00000000; 
        }
    }
    col = 15;
    row = 30;
}

uint32_t write_pointer(uint32_t x, uint32_t y, uint32_t color) {
    char *ptr = term_pointer;
    uint32_t glyph_h = psf1_get_height();
    uint32_t glyph_w = GLYPH_WIDTH;
    uint32_t clock_pos = (width - glyph_w * 10);
    
    while (*ptr) {
        draw_char(x, y, *ptr++, color);
        x += glyph_w;
    }

    char time[20];
    get_time(time);
    char *timee = time;
    while (*timee) {
        draw_char(clock_pos, y, *timee++, 0x0000FF00);
        clock_pos += glyph_w;
    }
    cursor_update(row + 10, x, 0x00FFFFFF);
    return x;
}

void termwrite(char ch, uint32_t color) {
    
    if (!opened && ch == '\n') {
        clear_term();
        opened = 1; 
    }

    if (!opened) return;

    cursor_update(row + 10, col + 8, 0x00FFFFFF);
    
    uint32_t glyph_h = psf1_get_height();
    
    if (ch == '\n') {
        key_counter[key_index] = '\0';    
        if (strcmp(key_counter, "clear") == 0) {
            clear_term();
        }
        
        if (strcmp(key_counter, "mdown") == 0) {
            shutdown();            
        }

        if (strcmp(key_counter, "reboot") == 0) {
            reboot();
        }
        col = 15;
        row += glyph_h;
        key_index = 0; 
        key_counter[0] = '\0';
        col = write_pointer(col, row, color); 
        return;
    }

    if (col + GLYPH_WIDTH > (width - 15)) { 
        col = 15;
        row += glyph_h;
    }
    if (row + glyph_h > (height - 10)) {
        row = 38;
        col = 15;
    }

    key_counter[key_index++] = ch;
    key_counter[key_index] = '\0'; 

    draw_char(col, row, ch, color);
    col += GLYPH_WIDTH;
}
