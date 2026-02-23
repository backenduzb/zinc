#include <kernel/io.h>
#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <stdint.h>
#include <kernel/string/str.h>
#include <string.h>

int current_offset = 0; 

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

typedef struct{
    const char* name;
    int offset;
} Timezone;

Timezone timezones[] = {
    {"Asia/Tashkent", 5},
    {"Europe/London", 0},
    {"Asia/Tokyo", 9},
    {"Amerika/NewYork", -5},
};

int tz_count = sizeof(timezones) / sizeof(Timezone); 

void set_timezone(int argc, char **argv){

    if (argc < 2){
        vga_set_color(VGA_YELLOW, VGA_BLACK);
        vga_write("\nWARNING: ");
        vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
        vga_write("Timezone was not writed user like this: set-timezone <Zone>\n");
        return;
    }

    for (int i = 0; i < tz_count; i++){
        if (strcmp(argv[1], timezones[i].name) == 0){
            current_offset = timezones[i].offset;
            vga_set_color(VGA_LIGHT_GREEN, VGA_BLACK);
            vga_write("\nTimezone setted successfully!\n");
            vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
            return;
        }
    }
    vga_write("\nTimezone ");
    vga_set_color(VGA_LIGHT_RED, VGA_BLACK);
    vga_write(argv[1]);
    vga_set_color(VGA_YELLOW, VGA_BLACK);
    vga_write(" NOT ");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("found");
}

void write_time() {
    uint8_t sec = bcd_to_dec(rtc_read(0x00));
    uint8_t min = bcd_to_dec(rtc_read(0x02));
    uint8_t hour = bcd_to_dec(rtc_read(0x04));
    hour = (hour + current_offset) % 24;
    if (hour < 0) hour += 24;
    
    char buff[9];
    char tmp[3];

    itoa2(hour, tmp);  buff[0] = tmp[0]; buff[1] = tmp[1]; buff[2] = ':';
    itoa2(min, tmp);   buff[3] = tmp[0]; buff[4] = tmp[1]; buff[5] = ':';
    itoa2(sec, tmp);   buff[6] = tmp[0]; buff[7] = tmp[1]; buff[8] = 0;
    
    vga_write_soat(vga_get_row(), 70, buff);
}

