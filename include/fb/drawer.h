#pragma once
#include <stdint.h>

extern uint32_t row;
extern uint32_t col;

void draw_line_white(void);
void draw_line(void);
void write(const char *text, uint32_t color);

void write_center(const char *text, uint32_t color);
void draw_screen_border(uint32_t color);
void draw_progress_bar(uint32_t x, uint32_t y, uint32_t width, uint32_t progress, uint32_t color);
void draw_spinner(uint32_t x, uint32_t y, uint32_t step, uint32_t color);
void draw_gradient_background(void);
void draw_os_name_animated(uint32_t step);
void show_splash_screen(void);
void write_center_with_duration(const char *text, uint32_t color, uint32_t duration);