#include <fb/drawer.h>
#include <timer/pit.h>
#include <time/time.h>
#include <stdint.h>
#include <ui/label.h>

void draw_waybar() {
    fill_rect(0, 0, 0, 30, 0x003b3b3b);
    center_label(15, "15:00:00", 0x00FFFFFF);
}