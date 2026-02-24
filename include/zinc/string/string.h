#pragma once
#include <stdint.h>
#include <stddef.h>

size_t utoa(uint64_t value, char* buffer);
size_t count(const char *str, const char sep);
size_t len(const char *str);