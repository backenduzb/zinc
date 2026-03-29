#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <stdint.h>
#include <ui/label.h>

static uint32_t last_ticks = 0;

void draw_waybar() {
    if (ticks - last_ticks < 1000) {
        return;
    }

    last_ticks += 1000;
    update_time();

    char time_bufer[9];
    get_time(time_bufer);
    fill_rect(0, 0, 0, 30, 0x001b1b1b);
    label(-20, 15, time_bufer, 0x001b1b1b);
    label(-20, 15, time_bufer, 0x00FFFFFF);
//    label(26, 15, "Sun 22 2026", 0x00FFFFFF);
} 
