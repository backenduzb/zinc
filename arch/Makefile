CC=clang
LD=ld.lld

CFLAGS=-target i686-elf -ffreestanding -O2 -Wall -Wextra -nostdlib -Ikernel/include
ASFLAGS=-target i686-elf
LDFLAGS=-T linker.ld -nostdlib

SRC=\
kernel/src/arch/x86/boot.s \
kernel/src/arch/x86/idt_load.s \
kernel/src/arch/x86/idt.c \
kernel/src/drivers/keyboard/keyboard.c \
kernel/src/arch/x86/isr.s \
kernel/src/arch/x86/pic.c \
kernel/src/arch/x86/io.c \
kernel/src/drivers/vga/vga.c \
kernel/src/drivers/vga/cursor.c \
kernel/src/kernel/main.c \

OBJ=$(SRC:.c=.o)
OBJ:=$(OBJ:.s=.o)

all: kernel.bin

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.s
	$(CC) $(ASFLAGS) -c $< -o $@

kernel.bin: $(OBJ)
	$(LD) $(LDFLAGS) $(OBJ) -o kernel.bin

iso: kernel.bin
	mkdir -p iso/boot/grub
	cp kernel.bin iso/boot/kernel.bin
	cp kernel/grub/grub.cfg iso/boot/grub/grub.cfg
	grub-mkrescue -o zinc.iso iso

run: iso
	qemu-system-i386 -cdrom zinc.iso

clean:
	rm -rf iso zinc.iso kernel.bin $(OBJ)