#pragma once
#include <stdint.h>

typedef struct {
    uint8_t* dsdt;
    int cpu_temp_offset;
} acpi_context_t;

extern acpi_context_t acpi;

void acpit_init();
int acpi_get_cpu_temp();