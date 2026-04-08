#include <disk/ata.h>
#include <io.h>

static void ata_wait_bsy(void) {
    while(inb(ATA_STATUS) & 0x80);
}

static void ara_wait_drq(void) {
    while(!(inb(ATA_STATUS) & 0x08));
}

static void ata_select_drive(unsigned int lba) {
    outb(ATA_DEVICE, 0xE0 | ((lba >> 2) & 0x0F));
}

void ata_read_sectors(unsigned int lba, unsigned char sectors, unsigned char *buf) {
    ata_wait_bsy();
    ata_select_drive(lba);
    
    outb(ATA_SEC_COUNT, sectors);
    outb(ATA_LBA_LOW, (lba & 0xFF));
    outb(ATA_LBA_MID, (lba >> 8) & 0xFF);
    outb(ATA_LBA_HIGH, (lba >> 16) & 0xFF);
    
    outb(ATA_COMMAND, ATA_CMD_READ_PIO);
    
    for (int i = 0; i<256; i++) {
        unsigned short word = inw(ATA_DATA);
        buf[i*2] = word & 0xFF;
        buf[i*2+1] = (word >> 8) & 0xFF;
    }
    buf += 512;
}

void ata_write_sectors(unsigned int lba, unsigned char sectors, unsigned char *buf) {
    ata_wait_bsy();
    ata_select_drive(lba);
    
    outb(ATA_SEC_COUNT, sectors);
    outb(ATA_LBA_LOW, (lba & 0xFF));
    outb(ATA_LBA_MID, (lba >> 8) & 0xFF);
    outb(ATA_LBA_HIGH, (lba >> 16) & 0xFF);
    outb(ATA_COMMAND, ATA_CMD_WRITE_PIO);
    
    for(int s=0; s < sectors; s++) {
        ata_wait_bsy();
        for (int i=0; i<256; i++){
            unsigned short word = buf[i*2] | (buf[i*2+1] << 8);
            outw(ATA_COMMAND, word);
        }
        buf+=512;
    }
    
    outb(ATA_COMMAND, 0xE7);
    ata_wait_bsy();
}

void ata_init(void) {
    outb(ATA_ALT_STATUS, 0x04);
    inb(ATA_STATUS);
    inb(ATA_ALT_STATUS);
    outb(ATA_ALT_STATUS, 0x00);
    
    ata_wait_bsy();
    outb(ATA_DEVICE, 0xA0);
    outb(ATA_SEC_COUNT, 0);
    outb(ATA_LBA_LOW, 0);
    outb(ATA_LBA_MID, 0);
    outb(ATA_LBA_HIGH, 0);
    outb(ATA_COMMAND, ATA_CMD_IDENTIFY);
    
    if (inb(ATA_STATUS) == 0) return;
    
    ata_wait_bsy();
    
    unsigned short indetify[256];
    for (int i=0; i<256; i++){
        indetify[i] = inw(ATA_DATA);
    }
}