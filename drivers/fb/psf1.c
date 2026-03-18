#include <stdint.h>
#include <fb/fb.h>
#include <font/psf1.h>

#define PSF1_MAGIC0 0x36
#define PSF1_MAGIC1 0x04
#define PSF1_MODE512 0x01

static const uint8_t *psf1_glyphs = 0;
static uint32_t psf1_charsize = 0;
static uint32_t psf1_glyph_count = 0;

void psf1_init(const void *font_data) {
    const psf1_header_t *hdr = (const psf1_header_t *)font_data;
    if (!hdr || hdr->magic[0] != PSF1_MAGIC0 || hdr->magic[1] != PSF1_MAGIC1) {
        psf1_glyphs = 0;
        psf1_charsize = 0;
        psf1_glyph_count = 0;
        return;
    }

    psf1_charsize = hdr->charsize;
    psf1_glyph_count = (hdr->mode & PSF1_MODE512) ? 512 : 256;
    psf1_glyphs = (const uint8_t *)(hdr + 1);
}

const uint8_t *get_glyph(char c) {
    if (!psf1_glyphs || psf1_charsize == 0) {
        return 0;
    }

    uint32_t index = (uint8_t)c;
    if (index >= psf1_glyph_count) {
        return 0;
    }

    return psf1_glyphs + (index * psf1_charsize);
}

void draw_char(int x, int y, char c, uint32_t color) {
    const uint8_t *glyph = get_glyph(c);
    if (!glyph || !framebuffer) {
        return;
    }
    if (x < 0 || y < 0) {
        return;
    }

    for (uint32_t row = 0; row < psf1_charsize; ++row) {
        uint32_t py = (uint32_t)y + row;
        if (py >= height) {
            break;
        }
        uint8_t bits = glyph[row];
        uint32_t *row_ptr = (uint32_t *)((uint8_t *)framebuffer + py * pitch);

        for (uint32_t col = 0; col < 8; ++col) {
            uint32_t px = (uint32_t)x + col;
            if (px >= width) {
                break;
            }
            if (bits & (uint8_t)(0x80 >> col)) {
                row_ptr[px] = color;
            }
        }
    }
}

uint32_t psf1_get_height(void) {
    return psf1_charsize;
}
