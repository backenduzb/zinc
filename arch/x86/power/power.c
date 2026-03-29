#include <stdint.h>
#include <power.h>
#include <io.h>

void reboot() {
    while(1) {
        uint8_t status;

        __asm__ volatile("inb %1, %0" : "=a"(status) : "Nd"(0x64));

        if ((status & 0x02) == 0) {
            break;
        }
    }

    outb(0x64, 0xFE);
}

void shutdown() {
    __asm__ volatile("outw %0, %1" : : "a"(0x2000), "Nd"(0x604));
}
