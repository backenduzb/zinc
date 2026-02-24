#pragma once
#include <stdint.h>
#include <stddef.h>

void framebuffer_init(void);
void put_pixel(size_t x, size_t y, uint32_t color);
void draw_char(size_t x, size_t y, char c, uint32_t color);
void draw_string(const char *str, uint32_t color);
void draw_border(uint32_t color, size_t thickness);
void framebuffer_init(void);

uint64_t framebuffer_width(void);
uint64_t framebuffer_height(void);

extern const char font8x8_basic[128][8]; 