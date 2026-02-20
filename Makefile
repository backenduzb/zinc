CC ?= clang
LD ?= ld.lld

export CC LD

.PHONY: all iso run clean

all:
	$(MAKE) -C arch all

iso:
	$(MAKE) -C arch iso

run: iso
	qemu-system-i386 -cdrom zinc.iso

clean:
	$(MAKE) -C arch clean
