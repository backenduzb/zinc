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
    uint16_t value = 0x2000;
    uint16_t port = 0x604;
    __asm__ volatile("outw %w0, %w1" : : "a"(value), "d"(port));
}
