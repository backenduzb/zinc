#include <kernel/io.h>
#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <stdint.h>

uint8_t rtc_read(uint8_t reg){
    outb(0x70, reg);
    return inb(0x71);
}

uint8_t bcd_to_dec(uint8_t val) {
    return ((val >> 4) * 10) + (val & 0x0F);
}

void itoa2(uint8_t val, char *buf) {
    buf[0] = '0' + val / 10;
    buf[1] = '0' + val % 10;
    buf[2] = 0;
}

void get_time() {
    uint8_t sec = bcd_to_dec(rtc_read(0x00));
    uint8_t min = bcd_to_dec(rtc_read(0x02));
    uint8_t hour = bcd_to_dec(rtc_read(0x04));

    char buff[9];
    char tmp[3];

    itoa2(hour, tmp);  buff[0] = tmp[0]; buff[1] = tmp[1]; buff[2] = ':';
    itoa2(min, tmp);   buff[3] = tmp[0]; buff[4] = tmp[1]; buff[5] = ':';
    itoa2(sec, tmp);   buff[6] = tmp[0]; buff[7] = tmp[1]; buff[8] = 0;

    vga_write_soat(vga_get_row(), 70, buff);
}
