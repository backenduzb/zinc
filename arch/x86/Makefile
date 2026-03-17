.PHONY: all boot cpu clean

all: arch.o

boot:
	$(MAKE) -C boot

cpu:
	$(MAKE) -C cpu

arch.o: boot cpu
	ld -r -o arch.o boot/boot.o boot/multiboot.o cpu/gdt.o cpu/paging.o cpu/idt.o cpu/isr.o

clean:
	rm -f arch.o
