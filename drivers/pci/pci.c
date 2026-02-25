#include <stdint.h>
#include <zinc/io.h>
#include <zinc/pci.h>
#include <zinc/video/framebuffer.h>

uint32_t pci_config_read(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset){
    uint32_t address =
            (1u << 31) |
            ((uint32_t)bus  << 16) |
            ((uint32_t)slot << 11) |
            ((uint32_t)func << 8)  |
            (offset & 0xFC);
    
        outl(0xCF8, address);
        return inl(0xCFC); 
}

void pci_scan_for_xhci(void)
{
    for (uint16_t bus = 0; bus < 256; bus++) {
        for (uint8_t slot = 0; slot < 32; slot++) {

            uint32_t vendor =
                pci_config_read(bus, slot, 0, 0x00);

            if ((vendor & 0xFFFF) == 0xFFFF)
                continue;

            uint32_t class_code =
                pci_config_read(bus, slot, 0, 0x08);

            uint8_t class    = (class_code >> 24) & 0xFF;
            uint8_t subclass = (class_code >> 16) & 0xFF;
            uint8_t prog_if  = (class_code >> 8)  & 0xFF;

            if (class == 0x0C &&
                subclass == 0x03 &&
                prog_if == 0x30)
            {
                draw_string("xHCI FOUND\n", 0x00FF00);
                return;
            }
        }
    }

    draw_string("xHCI NOT FOUND\n", 0xFF0000);
}

