.PHONY: all arch drivers kernel iso run clean

all: iso

arch:
	$(MAKE) -C arch/x86

drivers:
	$(MAKE) -C drivers/fb

kernel:
	$(MAKE) -C kernel

kernel.bin: arch kernel drivers linker.ld
	ld -m elf_x86_64 -n -o kernel.bin -T linker.ld arch/x86/arch.o kernel/main.o drivers/fb/fb.o

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
	rm -f kernel.bin os.iso
	rm -f *.o
