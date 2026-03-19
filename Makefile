.PHONY: all arch drivers kernel iso run clean utils

all: iso

arch:
	$(MAKE) -C arch/x86

drivers:
	$(MAKE) -C drivers/fb
	$(MAKE) -C drivers/input/keyboard
	$(MAKE) -C drivers/timer
	$(MAKE) -C drivers/time

kernel:
	$(MAKE) -C kernel

utils:
	$(MAKE) -C utils

kernel.bin: arch kernel drivers utils linker.ld
	ld -m elf_x86_64 -n -o kernel.bin -T linker.ld arch/x86/arch.o kernel/main.o kernel/idt.o drivers/fb/fb.o drivers/input/keyboard/keyboard.o drivers/timer/pic.o drivers/timer/pit.o drivers/time/time.o utils/utils.o

iso: kernel.bin
	mv kernel.bin iso/boot/kernel.bin
	grub-mkrescue -o os.iso iso

run: iso
	qemu-system-x86_64 -cdrom os.iso -boot d

clean:
	$(MAKE) -C arch/x86/boot clean
	$(MAKE) -C arch/x86/cpu clean
	$(MAKE) -C arch/x86 clean
	$(MAKE) -C kernel clean
	$(MAKE) -C drivers/fb clean
	$(MAKE) -C drivers/input/keyboard clean
	$(MAKE) -C drivers/timer clean
	$(MAKE) -C drivers/time clean
	$(MAKE) -C utils clean
	rm -f kernel.bin os.iso
	rm -f *.o
