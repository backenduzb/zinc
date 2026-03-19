#include <stdint.h>
#include <stddef.h>
#include <utils/string.h>

size_t strlen(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    return len;
}

void *memset(void *ptr, int value, size_t count) {
    uint8_t *p = (uint8_t *)ptr;
    while (count--) {
        *p++ = (uint8_t)value;
    }
    return ptr;
}

uint32_t uitoa(uint32_t value, char *buf) {
    uint32_t i = 0;
    do {
        buf[i++] = '0' + (value % 10);
        value /= 10;
    } while (value);
    for (uint32_t j = 0; j < i / 2; ++j) {
        char tmp = buf[j];
        buf[j] = buf[i - 1 - j];
        buf[i - 1 - j] = tmp;
    }
    buf[i] = '\0';
    return i;
}