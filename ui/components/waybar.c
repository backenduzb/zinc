#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <stdint.h>
#include <ui/label.h>

void draw_waybar() {
    fill_rect(0, 0, 0, 30, 0x001b1b1b);
    char time_bufer[9];
    get_time(time_bufer);
    center_label(15, time_bufer, 0x00FFFFFF);
    sleep(100);
    center_label(15, time_bufer, 0x001b1b1b);
}