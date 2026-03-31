#pragma once
#include <stddef.h>
#include <stdint.h>

int strcmp(const char *s1, const char *s2);
size_t strlen(const char *s);
void *memset(void *ptr, int value, size_t count);
uint32_t uitoa(uint32_t value, char *buf);
