#pragma once
#include <stdint.h>

typedef struct {
    uint8_t sec;
    uint8_t min;
    uint8_t hour;
    
    uint8_t dow;
    uint8_t day;
    uint8_t month;
    uint16_t year;
} Date;

void rtc_init(void);
void update_time(void);
void get_time(char *buff);
