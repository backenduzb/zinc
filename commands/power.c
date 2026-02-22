#include <stdint.h>

void shutdown(int argc, char **argv) {
    (void)argc;
    (void)argv;
    __asm__ __volatile__ (
        "mov $0x604, %%dx\n\t"
        "mov $0x2000, %%ax\n\t"
        "out %%ax, %%dx\n\t"
        "hlt\n\t"
        :
        :
        : "ax", "dx"
    );
}
void reboot(int argc, char **argv) {
    (void)argc;
    (void)argv;
    __asm__ __volatile__("cli");

    __asm__ __volatile__(
        "mov $0xFE, %al\n\t"
        "out %al, $0x64\n\t"
    );

    while (1) {
        __asm__ __volatile__("hlt");
    }
}
