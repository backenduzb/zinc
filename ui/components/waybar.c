#include <timer/pit.h>
#include <time/time.h>
#include <stdint.h>
#include <ui/layout.h>
#include <ui/label.h>

static uint32_t last_ticks = 0;

void draw_waybar() {
    if (ticks - last_ticks < 1000) {
        return;
    }

    last_ticks += 1000;
    update_time();

    char time_bufer[9];
    uint32_t bar_height = ui_get_topbar_height();
    int32_t right_padding = (int32_t)ui_get_padding();
    int32_t text_y = (int32_t)(bar_height / 2);

    get_time(time_bufer);
    fill_rect(0, 0, 0, bar_height, 0x001b1b1b);
    label(-(right_padding - 1), text_y + 1, time_bufer, 0x001b1b1b);
    label(-right_padding, text_y, time_bufer, 0x00FFFFFF);
}
