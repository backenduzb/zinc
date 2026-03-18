#include <stdint.h>
#include <fb/fb.h>
#include <fb/drawer.h>
#include <utils/string.h>
#include <timer/pit.h>

#define SPLASH_STEPS 100
#define SPLASH_DELAY_MS 10 


void draw_progress_bar(uint32_t x, uint32_t y, uint32_t width, uint32_t progress, uint32_t color) {
    uint32_t bar_width = (width * progress) / 100;
    for (uint32_t i = 0; i < width; i++) {
        framebuffer[y * (pitch / 4) + (x + i)] = 0x00333333; 
    }
    for (uint32_t i = 0; i < bar_width; i++) {
        framebuffer[y * (pitch / 4) + (x + i)] = color;
    }
    
    framebuffer[y * (pitch / 4) + x] = 0x00FFFFFF;
    framebuffer[y * (pitch / 4) + (x + width - 1)] = 0x00FFFFFF; 
}
void draw_spinner(uint32_t x, uint32_t y, uint32_t step, uint32_t color) {
    const char spinner_chars[] = {'|', '/', '-', ' '};
    uint32_t index = step % 4;
    
    for (int dy = 0; dy < 8; dy++) {
        for (int dx = 0; dx < 8; dx++) {
            framebuffer[(y + dy) * (pitch / 4) + (x + dx)] = 0x00000000;
        }
    }
    
    char spinner_str[2] = {spinner_chars[index], '\0'};
    uint32_t old_col = col;
    uint32_t old_row = row;
    col = x;
    row = y;
    write(spinner_str, color);
    col = old_col;
    row = old_row;
}

void draw_gradient_background(void) {
    for (uint32_t y = 0; y < height; y++) {
        uint32_t intensity = 255 - ((y * 255) / height);
        
        uint32_t color = (intensity << 16) | (intensity << 8) | intensity;
        
        for (uint32_t x = 0; x < width; x++) {
            framebuffer[y * (pitch / 4) + x] = color;
        }
    }
}

void draw_os_name_animated(uint32_t step) {
    const char *os_name = "ZINC";
    uint32_t len = strlen(os_name);
    uint32_t text_width = len * 8;
    uint32_t x = (width - text_width) / 2;
    uint32_t y = height / 3;

    uint32_t chars_to_show = (step * len) / SPLASH_STEPS;
    
    for (int dy = 0; dy < 8; dy++) {
        for (uint32_t dx = 0; dx < text_width; dx++) {
            framebuffer[(y + dy) * (pitch / 4) + (x + dx)] = 0x00000000;
        }
    }
    
    uint32_t old_col = col;
    uint32_t old_row = row;
    col = x;
    row = y;
    
    for (uint32_t i = 0; i < chars_to_show && i < len; i++) {
        char temp_str[2] = {os_name[i], '\0'};
        write(temp_str, 0x00FFFFFF);
    }
    
    col = old_col;
    row = old_row;
}

void show_splash_screen(void) {
    for (uint32_t i = 0; i < width * height; i++) {
        framebuffer[i] = 0x00000000;
    }
    draw_gradient_background();
    draw_screen_border(0x00FFFFFF);
    for (int step = 0; step <= SPLASH_STEPS; step++) {
        draw_os_name_animated(step);
        uint32_t old_col = col;
        uint32_t old_row = row;
        col = (width - 80) / 2;
        row = height * 2 / 3;
        write("Loading", 0x00FFFFFF);
        col = old_col;
        row = old_row;
        draw_spinner((width + 40) / 2, height * 2 / 3, step, 0x00FFFFFF);
        draw_progress_bar((width - 200) / 2, height * 2 / 3 + 20, 200, 
                         (step * 100) / SPLASH_STEPS, 0x0000FF00);
        old_col = col;
        old_row = row;
        col = 10;
        row = height - 20;
        write("Version 1.0.0", 0x00AAAAAA);
        col = old_col;
        row = old_row;
        col = width - 100;
        row = height - 20;
        write("(C) 2026", 0x00AAAAAA);
        col = old_col;
        row = old_row;
        sleep(SPLASH_DELAY_MS);
    }
    
    for (int i = 0; i < 30; i++) {
        draw_spinner((width + 40) / 2, height * 2 / 3, SPLASH_STEPS + i, 0x0000FF00);
            draw_progress_bar((width - 200) / 2, height * 2 / 3 + 20, 200, 100, 0x0000FF00);
        uint32_t old_col = col;
        uint32_t old_row = row;
        col = (width - 40) / 2;
        row = height * 2 / 3 + 40;
        write("Ready!", 0x0000FF00);
        col = old_col;
        row = old_row;
        
        sleep(100);
    }
    for (int i = 0; i < 10; i++) {
        for (uint32_t y = 0; y < height; y++) {
            for (uint32_t x = 0; x < width; x++) {
                uint32_t pixel = framebuffer[y * (pitch / 4) + x];
                uint32_t r = (pixel >> 16) & 0xFF;
                uint32_t g = (pixel >> 8) & 0xFF;
                uint32_t b = pixel & 0xFF;
                
                r = r * (9 - i) / 10;
                g = g * (9 - i) / 10;
                b = b * (9 - i) / 10;
                
                framebuffer[y * (pitch / 4) + x] = (r << 16) | (g << 8) | b;
            }
        }
        sleep(50);
    }
    for (uint32_t i = 0; i < width * height; i++) {
        framebuffer[i] = 0x00000000;
    }
}
