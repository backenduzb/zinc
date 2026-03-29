#include <fb/fb.h>
#include <font/psf1.h>
#include <ui/layout.h>
#include <stdint.h>

static uint32_t clamp_u32(uint32_t value, uint32_t min, uint32_t max) {
    if (value < min) {
        return min;
    }
    if (value > max) {
        return max;
    }
    return value;
}

uint32_t ui_get_padding(void) {
    if (width == 0) {
        return 12;
    }

    return clamp_u32(width / 64, 12, 48);
}

uint32_t ui_get_topbar_height(void) {
    uint32_t glyph_h = psf1_get_height();
    uint32_t vertical_padding = 6;

    if (height != 0) {
        vertical_padding = clamp_u32(height / 96, 6, 24);
    }

    if (glyph_h == 0) {
        glyph_h = 16;
    }

    return glyph_h + (vertical_padding * 2);
}

uint32_t ui_get_content_origin_x(void) {
    return ui_get_padding();
}

uint32_t ui_get_content_origin_y(void) {
    return ui_get_topbar_height() + ui_get_padding();
}
