#include <stdint.h>
#include <stddef.h>
#include <zinc/video/framebuffer.h>
#include <zinc/time/btime.h>
#include <zinc/interrupts/interrupts.h>
#include <zinc/interrupts/keyboard.h>
#include <zinc/pci.h>
#include <zinc/string/string.h>

extern uint8_t xhci_bus;
extern uint8_t xhci_slot;
extern uint8_t xhci_func;

void kernel_main(void) {
    framebuffer_init();
    draw_string_center("Welcome to ZINC OS!", 0x00FFFF);

    pci_scan_for_xhci(); 
    
    uint32_t bar0 = pci_config_read(xhci_bus, xhci_slot, xhci_func, 0x10);
    uint32_t bar1 = pci_config_read(xhci_bus, xhci_slot, xhci_func, 0x14);
    uint64_t mmio_base = ((uint64_t)bar1 << 32) | (bar0 & 0xFFFFFFF0);
    
    char buf[21];
    utoa(xhci_bus, buf); draw_string("BUS: ", 0xFF0000); draw_string(buf, 0xFF0000);
    utoa(xhci_slot, buf); draw_string(" SLOT: ", 0xFF0000); draw_string(buf, 0xFF0000);
    utoa(xhci_func, buf); draw_string(" FUNC: ", 0xFF0000); draw_string(buf, 0xFF0000);
    
    draw_string("\n/root%zinc > ", 0xFFFFFF);
    write_time();

    interrupts_init();
    interrupts_enable();

    while (1) {
        __asm__ volatile ("hlt");
    }
}