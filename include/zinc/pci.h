#pragma once
#include <stdint.h>

void pci_scan_for_xhci(void);
uint32_t pci_config_read(uint8_t bus, uint8_t slot, uint8_t funcn, uint8_t offset);