#include <stdint.h>
#include <stddef.h>
#include <utils/string.h>

size_t strlen(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    return len;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return *(unsigned char*)s1 - *(unsigned char*)s2;
}

void *memset(void *ptr, int value, size_t count) {
    uint8_t *p = (uint8_t *)ptr;
    while (count--) {
        *p++ = (uint8_t)value;
    }
    return ptr;
}

char *strchr(const char *str, int c) {
    while (1) {
        if (*str == (char)c) return (char*)str;
        if (*str == '\0') return NULL;
        str ++;
    }
}

char *strtok(char *str, const char *delim) {
    static char *next;

    if (str != NULL) {
        next = str;
    }

    if (next == NULL) return NULL;

    while (*next && strchr(delim, *next)) {
        next++;
    }

    if (*next == '\0') return NULL;

    char *start = next;

    while (*next && !strchr(delim, *next)) {
        next++;
    }

    if (*next) {
        *next = '\0';
        next++;
    }

    return start;
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
