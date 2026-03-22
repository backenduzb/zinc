#include <acpi.h>
#include <io.h>
#include <stdint.h>

typedef struct __attribute__((packed)) {
    char signature[8];
    uint8_t checksum;
    char oemid[6];
    uint8_t revision;
    uint32_t rsdt_addr;
} rsdp_descriptor_t;

typedef struct __attribute__((packed)) {
    rsdp_descriptor_t first;
    uint32_t length;
    uint64_t xsdt_addr;
    uint8_t extended_checksum;
    uint8_t reserved[3];
} rsdp_descriptor2_t;

typedef struct __attribute__((packed)) {
    char signature[4];
    uint32_t length;
    uint8_t revision;
    uint8_t checksum;
    char oemid[6];
    char oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
} acpi_sdt_header_t;

acpi_context_t acpi;

static int checksum_ok(const void *ptr, uint32_t len) {
    const uint8_t *bytes = (const uint8_t *)ptr;
    uint8_t sum = 0;
    for (uint32_t i = 0; i < len; i++) {
        sum = (uint8_t)(sum + bytes[i]);
    }
    return sum == 0;
}

static uint8_t *scan_rsdp_region(uint32_t start, uint32_t length) {
    const char sig[8] = { 'R','S','D',' ','P','T','R',' ' };
    for (uint32_t addr = start; addr < start + length; addr += 16) {
        const rsdp_descriptor_t *rsdp = (const rsdp_descriptor_t *)(uintptr_t)addr;
        int match = 1;
        for (int i = 0; i < 8; i++) {
            if (rsdp->signature[i] != sig[i]) {
                match = 0;
                break;
            }
        }
        if (!match) {
            continue;
        }
        if (!checksum_ok(rsdp, sizeof(rsdp_descriptor_t))) {
            continue;
        }
        return (uint8_t *)(uintptr_t)addr;
    }
    return 0;
}

static uint8_t *find_rsdp() {
    uint16_t ebda_seg = *(volatile uint16_t *)(uintptr_t)0x40E;
    uint32_t ebda_addr = (uint32_t)ebda_seg << 4;
    if (ebda_addr) {
        uint8_t *rsdp = scan_rsdp_region(ebda_addr, 1024);
        if (rsdp) {
            return rsdp;
        }
    }
    return scan_rsdp_region(0x000E0000, 0x20000);
}

static acpi_sdt_header_t *find_sdt(const acpi_sdt_header_t *sdt, const char sig[4]) {
    if (!sdt) {
        return 0;
    }
    uint32_t count;
    if (sdt->signature[0] == 'X') {
        count = (sdt->length - sizeof(acpi_sdt_header_t)) / 8;
        const uint64_t *entries = (const uint64_t *)((const uint8_t *)sdt + sizeof(acpi_sdt_header_t));
        for (uint32_t i = 0; i < count; i++) {
            acpi_sdt_header_t *hdr = (acpi_sdt_header_t *)(uintptr_t)entries[i];
            if (hdr->signature[0] == sig[0] && hdr->signature[1] == sig[1] &&
                hdr->signature[2] == sig[2] && hdr->signature[3] == sig[3]) {
                return hdr;
            }
        }
    } else {
        count = (sdt->length - sizeof(acpi_sdt_header_t)) / 4;
        const uint32_t *entries = (const uint32_t *)((const uint8_t *)sdt + sizeof(acpi_sdt_header_t));
        for (uint32_t i = 0; i < count; i++) {
            acpi_sdt_header_t *hdr = (acpi_sdt_header_t *)(uintptr_t)entries[i];
            if (hdr->signature[0] == sig[0] && hdr->signature[1] == sig[1] &&
                hdr->signature[2] == sig[2] && hdr->signature[3] == sig[3]) {
                return hdr;
            }
        }
    }
    return 0;
}

static void parse_dsdt(uint8_t *dsdt) {
    (void)dsdt;
    acpi.cpu_temp_offset = -1;
}

void acpit_init() {
    acpi.dsdt = 0;
    acpi.cpu_temp_offset = -1;

    uint8_t *rsdp_ptr = find_rsdp();
    if (!rsdp_ptr) {
        return;
    }

    const rsdp_descriptor_t *rsdp = (const rsdp_descriptor_t *)rsdp_ptr;
    const acpi_sdt_header_t *root = 0;

    if (rsdp->revision >= 2) {
        const rsdp_descriptor2_t *rsdp2 = (const rsdp_descriptor2_t *)rsdp_ptr;
        if (checksum_ok(rsdp2, rsdp2->length)) {
            root = (const acpi_sdt_header_t *)(uintptr_t)rsdp2->xsdt_addr;
        }
    }
    if (!root) {
        root = (const acpi_sdt_header_t *)(uintptr_t)rsdp->rsdt_addr;
    }
    if (!root || !checksum_ok(root, root->length)) {
        return;
    }

    const char facp_sig[4] = { 'F','A','C','P' };
    acpi_sdt_header_t *fadt = find_sdt(root, facp_sig);
    if (!fadt || !checksum_ok(fadt, fadt->length)) {
        return;
    }

    const uint8_t *fadt_bytes = (const uint8_t *)fadt;
    uint32_t dsdt = *(const uint32_t *)(fadt_bytes + 40);
    uint64_t x_dsdt = 0;
    if (fadt->length >= 148) {
        x_dsdt = *(const uint64_t *)(fadt_bytes + 140);
    }

    if (x_dsdt) {
        acpi.dsdt = (uint8_t *)(uintptr_t)x_dsdt;
    } else {
        acpi.dsdt = (uint8_t *)(uintptr_t)dsdt;
    }

    if (acpi.dsdt) {
        parse_dsdt(acpi.dsdt);
    }
}

int acpi_get_cpu_temp() {
    return -1;
}
