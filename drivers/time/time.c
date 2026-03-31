#include <io.h>
#include <time/time.h>
#include <stdint.h>
#include <timer/pit.h>

Date current_time;
uint64_t last_ticks = 0;

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
    current_time.sec = bcd_decoder(get_time_bcd(0x00));
    current_time.min = bcd_decoder(get_time_bcd(0x02));
    current_time.hour = bcd_decoder(get_time_bcd(0x04));
    current_time.hour = (current_time.hour + 5) % 24;
}

void update_time() {
    if (ticks - last_ticks >= 1000) {
        current_time.sec++;
        if (current_time.sec >= 60) { current_time.sec = 0; current_time.min++; }
        if (current_time.min == 60) { current_time.min = 0; current_time.hour++; }
        if (current_time.hour == 24) { current_time.hour = 0; }
        last_ticks = ticks;
    }
}

void get_time(char *buff) {
    char tmp[3];
    get_string(current_time.hour, tmp); buff[0] = tmp[0]; buff[1] = tmp[1]; buff[2] = ':';
    get_string(current_time.min, tmp); buff[3] = tmp[0]; buff[4] = tmp[1]; buff[5] = ':';
    get_string(current_time.sec, tmp); buff[6] = tmp[0]; buff[7] = tmp[1]; buff[8] = 0;
}
