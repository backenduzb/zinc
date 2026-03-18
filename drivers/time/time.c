#include <io.h>
#include <time/time.h>
#include <stdint.h>

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

void get_time(char *buff) {
    uint8_t sec = bcd_decoder(get_time_bcd(0x00));
    uint8_t min = bcd_decoder(get_time_bcd(0x02));
    uint8_t hour = bcd_decoder(get_time_bcd(0x04));

    hour = hour % 24;

    if (hour < 0) hour += 24;

    char tmp[3];

    get_string(hour, tmp); buff[0] = tmp[0]; buff[1] = tmp[1]; buff[2] = ':';
    get_string(min, tmp); buff[3] = tmp[0]; buff[4] = tmp[1]; buff[5] = ':';
    get_string(sec, tmp); buff[6] = tmp[0]; buff[7] = tmp[1]; buff[8] = 0;
}
