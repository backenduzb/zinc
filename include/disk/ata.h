#pragma once

#include <io.h>

#define ATA_DATA 0x1F0
#define ATA_ERROR 0x1F1
#define ATA_SEC_COUNT  0x1F2
#define ATA_LBA_LOW 0x1F3
#define ATA_LBA_MID 0x1F4
#define ATA_LBA_HIGH 0x1F5
#define ATA_DEVICE 0x1F6
#define ATA_COMMAND 0x1F7
#define ATA_STATUS 0x1F7
#define ATA_ALT_STATUS 0x3F6

#define ATA_CMD_READ_PIO 0x20
#define ATA_CMD_WRITE_PIO 0x30
#define ATA_CMD_IDENTIFY 0xEC

void ata_init(void);
void ata_read_sectors(unsigned int lba, unsigned char sectors, unsigned char *buf);
void ata_write_sectors(unsigned int lba, unsigned char sectors, unsigned char *buf);