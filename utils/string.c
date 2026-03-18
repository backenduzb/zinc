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
