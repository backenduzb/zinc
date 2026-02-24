#include <zinc/string/string.h>
#include <stdint.h>
#include <stddef.h>

size_t utoa(uint64_t value, char* buffer) {
    char temp[20];
    size_t i = 0;

    if (value == 0) {
        buffer[0] = '0';
        buffer[1] = '\0';
        return 1;
    }

    while (value > 0) {
        temp[i++] = '0' + (value % 10);
        value /= 10;
    }

    size_t len = i;
    for (size_t j = 0; j < len; j++) {
        buffer[j] = temp[len - j - 1];
    }

    buffer[len] = '\0';
    return len;
}

size_t count(const char *str, const char sep) {
    size_t count = 0;
    while (*str) {
        if (*str == sep) count++;
        str++;
    }
    return count;
}

size_t len(const char *str) {
    size_t count = 0;
    while (*str) {
        count++;
        str++;
    }
    return count;
}