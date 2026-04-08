#include <disk/ata.h>
#include <io.h>

#define ATA_POLL_LIMIT 1000000u

static uint16_t ata_identify_data[256];

static void ata_io_wait(void) {
    inb(ATA_ALT_STATUS);
    inb(ATA_ALT_STATUS);
    inb(ATA_ALT_STATUS);
    inb(ATA_ALT_STATUS);
}

static int ata_wait_not_busy(void) {
    for (uint32_t i = 0; i < ATA_POLL_LIMIT; ++i) {
        uint8_t status = inb(ATA_STATUS);

        if (status == 0xFF) {
            return -1;
        }

        if ((status & ATA_STATUS_BSY) == 0) {
            return 0;
        }
    }

    return -1;
}

static int ata_wait_drq(void) {
    for (uint32_t i = 0; i < ATA_POLL_LIMIT; ++i) {
        uint8_t status = inb(ATA_STATUS);

        if (status == 0xFF) {
            return -1;
        }

        if (status & (ATA_STATUS_ERR | ATA_STATUS_DF)) {
            return -1;
        }

        if (status & ATA_STATUS_DRQ) {
            return 0;
        }
    }

    return -1;
}

static void ata_select_drive(uint32_t lba) {
    outb(ATA_DEVICE, 0xE0u | (uint8_t)((lba >> 24) & 0x0Fu));
}

static void ata_setup_lba(uint32_t lba, uint8_t sectors) {
    outb(ATA_SEC_COUNT, sectors);
    outb(ATA_LBA_LOW, (uint8_t)(lba & 0xFFu));
    outb(ATA_LBA_MID, (uint8_t)((lba >> 8) & 0xFFu));
    outb(ATA_LBA_HIGH, (uint8_t)((lba >> 16) & 0xFFu));
}

static int ata_prepare_command(uint32_t lba, uint8_t sectors, uint8_t command) {
    if (sectors == 0) {
        return -1;
    }

    if (ata_wait_not_busy() != 0) {
        return -1;
    }

    ata_select_drive(lba);
    ata_setup_lba(lba, sectors);
    outb(ATA_COMMAND, command);
    ata_io_wait();

    return ata_wait_not_busy();
}

int ata_read_sectors(uint32_t lba, uint8_t sectors, uint8_t *buf) {
    if (buf == 0 || sectors == 0) {
        return -1;
    }

    for (uint8_t sector = 0; sector < sectors; ++sector) {
        uint32_t current_lba = lba + sector;
        uint8_t *sector_buf = buf + ((uint32_t)sector * ATA_SECTOR_SIZE);

        if (ata_prepare_command(current_lba, 1, ATA_CMD_READ_PIO) != 0) {
            return -1;
        }

        if (ata_wait_drq() != 0) {
            return -1;
        }

        for (uint32_t word = 0; word < ATA_SECTOR_SIZE / 2; ++word) {
            uint16_t data = inw(ATA_DATA);
            sector_buf[word * 2] = (uint8_t)(data & 0xFFu);
            sector_buf[word * 2 + 1] = (uint8_t)((data >> 8) & 0xFFu);
        }
    }

    return 0;
}

int ata_write_sectors(uint32_t lba, uint8_t sectors, const uint8_t *buf) {
    if (buf == 0 || sectors == 0) {
        return -1;
    }

    for (uint8_t sector = 0; sector < sectors; ++sector) {
        uint32_t current_lba = lba + sector;
        const uint8_t *sector_buf = buf + ((uint32_t)sector * ATA_SECTOR_SIZE);

        if (ata_prepare_command(current_lba, 1, ATA_CMD_WRITE_PIO) != 0) {
            return -1;
        }

        if (ata_wait_drq() != 0) {
            return -1;
        }

        for (uint32_t word = 0; word < ATA_SECTOR_SIZE / 2; ++word) {
            uint16_t data = (uint16_t)sector_buf[word * 2];
            data |= (uint16_t)sector_buf[word * 2 + 1] << 8;
            outw(ATA_DATA, data);
        }
    }

    outb(ATA_COMMAND, ATA_CMD_CACHE_FLUSH);
    ata_io_wait();

    return ata_wait_not_busy();
}

int ata_init(void) {
    uint8_t status = inb(ATA_STATUS);

    if (status == 0 || status == 0xFF) {
        return -1;
    }

    if (ata_wait_not_busy() != 0) {
        return -1;
    }

    outb(ATA_DEVICE, 0xA0u);
    ata_setup_lba(0, 0);
    outb(ATA_COMMAND, ATA_CMD_IDENTIFY);
    ata_io_wait();

    status = inb(ATA_STATUS);
    if (status == 0 || status == 0xFF) {
        return -1;
    }

    while (status & ATA_STATUS_BSY) {
        status = inb(ATA_STATUS);
    }

    if (status & ATA_STATUS_ERR) {
        return -1;
    }

    if ((status & ATA_STATUS_DRQ) == 0) {
        return -1;
    }

    for (uint32_t i = 0; i < 256; ++i) {
        ata_identify_data[i] = inw(ATA_DATA);
    }

    return 0;
}
