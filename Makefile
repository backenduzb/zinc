TARGET := x86_64-unknown-none
BUILD := build
ISO_ROOT := $(BUILD)/iso
KERNEL := $(ISO_ROOT)/boot/kernel.elf
ISO := $(BUILD)/zincos.iso
C_WORKSPACE = ./c
ASM_WORKSPACE = ./boot

RUSTFLAGS := -C relocation-model=static

.PHONY: all kernel iso run clean

all: iso

$(BUILD):
	mkdir -p $(BUILD)

asmbuild: $(ASM_WORKSPACE)
	make -C $(ASM_WORKSPACE)
	
cbuild: $(C_WORKSPACE)
	make -C $(C_WORKSPACE)
		
kernel:
	cargo build \
		-Z build-std=core,alloc \
		--target $(TARGET) \
		--release
		
$(KERNEL): kernel cbuild asmbuild
	mkdir -p $(ISO_ROOT)/boot

	ld \
		-nostdlib \
		-z max-page-size=0x1000 \
		-T linker.ld \
		-o $(KERNEL) \
		$(ASM_WORKSPACE)/asmbuild.o \
		target/$(TARGET)/release/libzincos.a \
		$(C_WORKSPACE)/cbuild.o

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
	make -C $(C_WORKSPACE) clean
	make -C $(ASM_WORKSPACE) clean