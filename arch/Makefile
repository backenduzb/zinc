ARCH ?= i686-elf

ifeq ($(origin CC), environment)
undefine CC
endif
ifeq ($(origin LD), environment)
undefine LD
endif
ifeq ($(origin CFLAGS), environment)
undefine CFLAGS
endif
ifeq ($(origin ASFLAGS), environment)
undefine ASFLAGS
endif
ifeq ($(origin LDFLAGS), environment)
undefine LDFLAGS
endif

ifneq ($(shell command -v $(ARCH)-gcc 2>/dev/null),)
CC ?= $(ARCH)-gcc
LD ?= $(ARCH)-ld
CFLAGS ?= -ffreestanding -O2 -Wall -Wextra -nostdlib -Ikernel/include
ASFLAGS ?=
else
CC ?= clang
LD ?= ld.lld
CFLAGS ?= -target $(ARCH) -ffreestanding -O2 -Wall -Wextra -nostdlib -Ikernel/include
ASFLAGS ?= -target $(ARCH)
endif

LDFLAGS ?= -T linker.ld -nostdlib

ifeq ($(shell command -v $(CC) 2>/dev/null),)
$(error Compiler '$(CC)' not found. Install clang or $(ARCH)-gcc, or run make CC=...)
endif

SRC = \
	kernel/src/arch/x86/boot.s \
	kernel/src/arch/x86/idt_load.s \
	kernel/src/arch/x86/isr.s \
	kernel/src/arch/x86/idt.c \
	kernel/src/arch/x86/pic.c \
	kernel/src/arch/x86/io.c \
	kernel/src/kernel/main.c \
	kernel/src/utils/string.c \
	../drivers/keyboard/keyboard.c \
	../drivers/keyboard/keyreader.c \
	../drivers/vga/vga.c \
	../drivers/vga/cursor.c \
	../drivers/time/btime.c \
	../commands/commands.c \
	../commands/power.c \

OBJ = $(SRC:.c=.o)
OBJ := $(OBJ:.s=.o)

.PHONY: all iso run clean

all: kernel.bin

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.s
	$(CC) $(ASFLAGS) -c $< -o $@

kernel.bin: $(OBJ)
	$(LD) $(LDFLAGS) $(OBJ) -o $@

iso: kernel.bin
	mkdir -p iso/boot/grub
	cp kernel.bin iso/boot/kernel.bin
	cp kernel/grub/grub.cfg iso/boot/grub/grub.cfg
	grub-mkrescue -o ../zinc.iso iso


clean:
	rm -f $(OBJ) kernel.bin zinc.iso
	rm -f iso/boot/kernel.bin
	rm -f ../zinc.iso
