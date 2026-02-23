#include <stdint.h>
#include <stddef.h>
#include <limine.h>

__attribute__((used, section(".limine_requests_start_marker")))
static volatile uint64_t limine_requests_start_marker[] = LIMINE_REQUESTS_START_MARKER;

__attribute__((used, section(".limine_requests")))
static volatile uint64_t limine_base_revision[] = LIMINE_BASE_REVISION(5);

__attribute__((used, section(".limine_requests")))
static volatile struct limine_framebuffer_request limine_framebuffer_request = {
    .id = LIMINE_FRAMEBUFFER_REQUEST_ID,
    .revision = 0
};

__attribute__((used, section(".limine_requests_end_marker")))
static volatile uint64_t limine_requests_end_marker[] = LIMINE_REQUESTS_END_MARKER;
static const uint8_t font8x8_basic[128][8] = {
    ['A'] = { 0x18,0x24,0x42,0x7E,0x42,0x42,0x42,0x00 },
    // ['B'] = { ... },
};

static uintptr_t fb_addr = 0;
static uint64_t fb_width = 0;
static uint64_t fb_height = 0;
static uint64_t fb_pitch = 0;
static uint16_t fb_bpp = 0;

static inline void put_pixel(size_t x, size_t y, uint32_t color) {
    if (!fb_addr || !fb_pitch || !fb_bpp) return;
    if (x >= fb_width || y >= fb_height) return;

    uint32_t bytes_per_pixel = fb_bpp / 8;
    uint8_t *p = (uint8_t *)(fb_addr + y * fb_pitch + x * bytes_per_pixel);

    switch (fb_bpp) {
        case 32:
            *(uint32_t*)p = color;
            break;
        case 24:
            p[0] = color & 0xFF;
            p[1] = (color >> 8) & 0xFF;
            p[2] = (color >> 16) & 0xFF;
            break;
        case 16:
            *(uint16_t*)p = (uint16_t)color;
            break;
    }
}

static void draw_char(size_t x, size_t y, char c, uint32_t color) {
    for (size_t i = 0; i < 8; i++) {
        uint8_t row = font8x8_basic[(uint8_t)c][i];
        for (size_t j = 0; j < 8; j++) {
            if (row & (1 << j)) put_pixel(x+j, y+i, color);
        }
    }
}

static void draw_string(size_t x, size_t y, const char *str, uint32_t color) {
    size_t px = x;
    while (*str) {
        draw_char(px, y, *str, color); 
        px += 8;
        str++;
    }
}

void kernel_main(void) {
    struct limine_framebuffer_response *fb_resp = limine_framebuffer_request.response;
    if (fb_resp && fb_resp->framebuffer_count > 0) {
        struct limine_framebuffer *fb = fb_resp->framebuffers[0];
        fb_addr = (uintptr_t)fb->address;
        fb_pitch = fb->pitch;
        fb_width = fb->width;
        fb_height = fb->height;
        fb_bpp = fb->bpp;
    }

    uint32_t white = 0x00FFFFFF;

    if (fb_addr) {
        draw_string(0, 0, "Hasc", white);
    }

    while (1) {
        __asm__ volatile ("hlt");
    }
}
