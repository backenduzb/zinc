.PHONY: all arch drivers kernel utils iso run clean

SUBDIRS := arch drivers kernel utils
SUBDIR_OBJS := $(foreach d,$(SUBDIRS),$(d)/$(notdir $(d)).o)

OUT := kernel.elf
ISO_KERNEL := iso/boot/kernel.elf

all: $(OUT)

arch:
	$(MAKE) -C arch

drivers:
	$(MAKE) -C drivers

kernel:
	$(MAKE) -C kernel

utils:
	$(MAKE) -C utils

$(SUBDIR_OBJS):
	$(MAKE) -C $(dir $@)

$(OUT): $(SUBDIR_OBJS) linker.ld
	ld -m elf_x86_64 -n -o $(OUT) -T linker.ld $(SUBDIR_OBJS)

iso: $(OUT)
	mkdir -p iso/boot
	cp $(OUT) $(ISO_KERNEL)
	grub-mkrescue -o os.iso iso

run: iso
	qemu-system-x86_64 -cdrom os.iso -boot d

clean:
	rm -f iso/boot/$(OUT)
	rm -f $(OUT) os.iso
	rm -f *.o
	for dir in $(SUBDIRS); do $(MAKE) -C $$dir clean; done
