#include <io.h>
#include <time/time.h>
#include <stdint.h>
#include <timer/pit.h>

uint32_t last_ticks = 0;
uint8_t sec = 0;
uint8_t min = 0;
uint8_t hour = 0;

uint8_t get_time_bcd(uint8_t reg) {
    outb(0x70, reg);
    return inb(0x71);
}

uint8_t bcd_decoder(uint8_t val) {
    return ((val >> 4) * 10 + (val & 0x0F));
}

void get_string(uint8_t val, char *buf) {
    buf[0] = '0' + val / 10;
    buf[1] = '0' + val % 10;
    buf[2] = 0;
}

void rtc_init() {
    sec = bcd_decoder(get_time_bcd(0x00));
    min = bcd_decoder(get_time_bcd(0x02));
    hour = bcd_decoder(get_time_bcd(0x04));
    hour = (hour + 5) % 24;
}

void update_time() {
    if (ticks - last_ticks >= 1000) {
        sec++;
        if (sec >= 60) { sec = 0; min++; }
        if (min == 60) { min = 0; hour++; }
        if (hour == 24) { hour = 0; }
        last_ticks = ticks;
    }
}

void get_time(char *buff) {
    char tmp[3];
    get_string(hour, tmp); buff[0] = tmp[0]; buff[1] = tmp[1]; buff[2] = ':';
    get_string(min, tmp); buff[3] = tmp[0]; buff[4] = tmp[1]; buff[5] = ':';
    get_string(sec, tmp); buff[6] = tmp[0]; buff[7] = tmp[1]; buff[8] = 0;
}
