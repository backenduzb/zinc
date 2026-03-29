#include <stdint.h>
#include <fb/fb.h>
#include <utils/string.h>
#include <timer/pit.h>

#define SPLASH_STEPS 50
#define SPLASH_DELAY_MS 2 

static inline uint32_t scale_intensity(uint32_t intensity, uint32_t step) {
    if (SPLASH_STEPS == 0) {
        return intensity;
    }
    if (step > SPLASH_STEPS) {
        step = SPLASH_STEPS;
    }
    return (uint32_t)(((uint64_t)intensity * step) / SPLASH_STEPS);
}

static void render_gradient(uint32_t brightness_step) {
    for (uint32_t y = 0; y < height; ++y) {
        uint32_t intensity = 255;
        uint32_t scaled = scale_intensity(intensity, brightness_step);
        uint32_t color = (scaled << 16) | (scaled << 8) | scaled;
        uint32_t row_start = y * (pitch / 4);

        for (uint32_t x = 0; x < width; ++x) {
            framebuffer[row_start + x] = color;
        }
    }
}

void draw_gradient_background(void) {
    render_gradient(SPLASH_STEPS);
}

void show_splash_screen(void) {
    for (uint32_t step = 0; step <= SPLASH_STEPS; ++step) {
        render_gradient(step);
        sleep(SPLASH_DELAY_MS);
    }

    if (SPLASH_STEPS == 0) {
        memset(framebuffer, 0, width * height * sizeof(uint32_t));
        return;
    }

    for (int32_t step = (int32_t)SPLASH_STEPS - 1; step >= 0; --step) {
        render_gradient((uint32_t)step);
        sleep(SPLASH_DELAY_MS);
    }

    memset(framebuffer, 0, width * height * sizeof(uint32_t));
}
