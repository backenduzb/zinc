#include <fb/fb.h>
#include <stdint.h>
#include <font/psf1.h>
#include <timer/pit.h>
#include <utils/string.h>

uint32_t *framebuffer = 0;
uint32_t pitch = 0;
uint32_t width = 0;
uint32_t height = 0;

static uint32_t font_height(void) {
    return psf1_get_height();
}

void init_framebuffer(uint64_t mbi_addr) {

    const struct multiboot_tag *tag =
        (const struct multiboot_tag *)((uintptr_t)mbi_addr + 8);

    for (; tag->type != MULTIBOOT_TAG_TYPE_END;
         tag = (const struct multiboot_tag *)((const uint8_t *)tag + ((tag->size + 7) & ~7))) {

        if (tag->type == MULTIBOOT_TAG_TYPE_FRAMEBUFFER) {

            const struct multiboot_tag_framebuffer *fb_tag =
                (const struct multiboot_tag_framebuffer *)tag;

            framebuffer = (uint32_t *)(uintptr_t)fb_tag->fb_addr;
            pitch = fb_tag->fb_pitch;
            width = fb_tag->fb_width;
            height = fb_tag->fb_height;

            break;
        }
    }
}
