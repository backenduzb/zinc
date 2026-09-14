TARGET := x86_64-unknown-none
BUILD := build
ISO_ROOT := $(BUILD)/iso
KERNEL := $(ISO_ROOT)/boot/kernel.elf
ISO := $(BUILD)/zincos.iso

RUSTFLAGS := -C relocation-model=static

.PHONY: all kernel iso run clean

all: iso

$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/entry.o: boot/entry.asm
	mkdir -p $(BUILD)
	nasm -f elf64 $< -o$@

$(BUILD)/paging.o: boot/paging.asm
	mkdir -p $(BUILD)
	nasm -f elf64 $< -o$@

	
$(BUILD)/test.o: c/hello.c
	mkdir -p $(BUILD)
	gcc \
		-m64 \
		-ffreestanding \
		-fno-stack-protector \
		-fno-pic \
		-fno-pie \
		-c $< \
		-o $@
		
kernel:
	cargo build \
		-Z build-std=core,alloc \
		--target $(TARGET) \
		--release
		
$(KERNEL): kernel $(BUILD)/entry.o $(BUILD)/test.o $(BUILD)/paging.o
	mkdir -p $(ISO_ROOT)/boot

	ld \
		-nostdlib \
		-z max-page-size=0x1000 \
		-T linker.ld \
		-o $(KERNEL) \
		$(BUILD)/entry.o \
		$(BUILD)/paging.o \
		target/$(TARGET)/release/libzincos.a \
		$(BUILD)/test.o

iso: $(KERNEL)
	mkdir -p $(ISO_ROOT)/boot

	cp limine.conf $(ISO_ROOT)/boot/
	cp limine/bin/limine-bios-cd.bin $(ISO_ROOT)/boot/
	cp limine/bin/limine-bios.sys $(ISO_ROOT)/boot/

	xorriso -as mkisofs \
		-b boot/limine-bios-cd.bin \
		-no-emul-boot \
		-boot-load-size 4 \
		-boot-info-table \
		$(ISO_ROOT) \
		-o $(ISO)

	limine/bin/limine bios-install $(ISO) --force

run: iso
	qemu-system-x86_64 \
		-cdrom $(ISO) \
		-m 512M \
		-serial stdio \
		-smp 4 \
		-boot d

clean:
	rm -rf $(BUILD)