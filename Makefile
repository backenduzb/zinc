.PHONY: all arch drivers kernel iso run clean

all: iso

arch:
	$(MAKE) -C arch/x86

drivers:
	$(MAKE) -C drivers/fb
	$(MAKE) -C drivers/timer

kernel:
	$(MAKE) -C kernel

kernel.bin: arch kernel drivers linker.ld
	ld -m elf_x86_64 -n -o kernel.bin -T linker.ld arch/x86/arch.o kernel/main.o kernel/idt.o drivers/fb/fb.o drivers/timer/pic.o drivers/timer/pit.o

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
	$(MAKE) -C drivers/timer clean
	rm -f kernel.bin os.iso
	rm -f *.o
