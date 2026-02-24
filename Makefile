SUBDIRS = kernel arch iso utils drivers

TARGET = kernel.elf
ISO_DIR = iso
ISO_BOOT = $(ISO_DIR)/boot
ISO_LIMINE = $(ISO_BOOT)/limine
ISO_EFI = $(ISO_DIR)/EFI/BOOT
ISO_KERNEL = $(ISO_BOOT)/kernel.elf
ISO_IMAGE = zinc.iso

LD = x86_64-elf-ld
LDFLAGS = -nostdlib -T linker.ld

LIMINE_DIR = limine
LIMINE_CONF = limine.conf
LIMINE_UEFI_CD = $(LIMINE_DIR)/bin/limine-uefi-cd.bin
LIMINE_EFI = $(LIMINE_DIR)/bin/BOOTX64.EFI

all: $(ISO_IMAGE)

$(TARGET):
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir; \
	done
	$(LD) $(LDFLAGS) arch/x86_64/boot.o kernel/*.o kernel/video/framebuffer.o utils/string.o drivers/time/btime.o -o $(TARGET)

$(ISO_IMAGE): $(TARGET)
	mkdir -p $(ISO_BOOT) $(ISO_LIMINE) $(ISO_EFI)
	cp $(TARGET) $(ISO_KERNEL)
	cp $(LIMINE_CONF) $(ISO_LIMINE)/limine.conf
	cp $(LIMINE_UEFI_CD) $(ISO_LIMINE)/
	cp $(LIMINE_EFI) $(ISO_EFI)/
	xorriso -as mkisofs -R -r -J -hfsplus -apm-block-size 2048 \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_DIR) -o $(ISO_IMAGE)

clean:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
	rm -f $(TARGET)
	rm -f $(ISO_IMAGE)