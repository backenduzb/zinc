#include <io.h>
#include <pic.h>
#include <stdint.h>
#include <timer/pit.h>

volatile uint64_t ticks = 0;

void timer_handler(void) {
    ticks++;
    pic_send_eoi(0);
}

void pit_init(uint32_t freq) {
    if (freq == 0) {
        freq = 1000;
    }

    uint32_t divisor = 1193182 / freq;
    if (divisor == 0) {
        divisor = 1;
    }

    outb(0x43, 0x36);

    outb(0x40, divisor & 0xFF);
    outb(0x40, (divisor >> 8) & 0xFF);
}

void sleep(uint64_t ms) {
    uint64_t end = ticks + ms;
    while (ticks < end) {
        __asm__ volatile("sti; hlt");
    }
}
