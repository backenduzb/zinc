#pragma once
#include <stdint.h>

typedef struct __attribute__((packed)) {
    uint8_t magic[2];
    uint8_t mode;
    uint8_t charsize;
} psf1_header_t;

void psf1_init(const void *font_data);
const uint8_t *get_glyph(char c);
void draw_char(int x, int y, char c, uint32_t color);
void draw_char_restore_bg(int x, int y, char c, uint32_t color);
uint32_t psf1_get_height(void);
