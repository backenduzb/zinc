ARCH ?= i686-elf

ifneq ($(origin LD), command line)
ifneq ($(shell command -v $(ARCH)-ld 2>/dev/null),)
LD := $(ARCH)-ld
else ifneq ($(shell command -v ld.lld 2>/dev/null),)
LD := ld.lld
else
LD := ld
endif
endif

LDFLAGS ?= -T arch/linker.ld -nostdlib -m elf_i386

ifeq ($(shell command -v $(LD) 2>/dev/null),)
$(error Linker '$(LD)' not found. Install llvm or $(ARCH)-binutils, or run make LD=...)
endif

KERNEL_BIN := arch/kernel.bin

ARCH_OBJ = \
	arch/kernel/src/arch/x86/boot.o \
	arch/kernel/src/arch/x86/idt_load.o \
	arch/kernel/src/arch/x86/isr.o \
	arch/kernel/src/arch/x86/idt.o \
	arch/kernel/src/arch/x86/pic.o \
	arch/kernel/src/arch/x86/io.o \
	arch/kernel/src/kernel/main.o \
	arch/kernel/src/utils/string.o \

DRIVERS_OBJ = \
	drivers/keyboard/keyboard.o \
	drivers/keyboard/keyreader.o \
	drivers/vga/vga.o \
	drivers/vga/cursor.o \
	drivers/time/btime.o \

COMMANDS_OBJ = \
	commands/commands.o \
	commands/power.o

ALL_OBJ := $(ARCH_OBJ) $(DRIVERS_OBJ) $(COMMANDS_OBJ)

.PHONY: all iso run clean drivers commands arch

all: $(KERNEL_BIN)

drivers:
	$(MAKE) -C drivers all ARCH=$(ARCH)

commands:
	$(MAKE) -C commands all ARCH=$(ARCH)

arch:
	$(MAKE) -C arch all ARCH=$(ARCH)

$(KERNEL_BIN): drivers commands arch
	$(LD) $(LDFLAGS) $(ALL_OBJ) -o $@

iso: $(KERNEL_BIN)
	mkdir -p arch/iso/boot/grub
	cp $(KERNEL_BIN) arch/iso/boot/kernel.bin
	cp arch/kernel/grub/grub.cfg arch/iso/boot/grub/grub.cfg
	grub-mkrescue -o zinc.iso arch/iso

run: iso
	qemu-system-x86_64 -cdrom ./zinc.iso -m 512M -serial stdio

clean:
	$(MAKE) -C arch clean ARCH=$(ARCH)
	$(MAKE) -C drivers clean ARCH=$(ARCH)
	$(MAKE) -C commands clean ARCH=$(ARCH)
	rm -f $(KERNEL_BIN) zinc.iso
	rm -f arch/iso/boot/kernel.bin
