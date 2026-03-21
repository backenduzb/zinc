ASM = nasm
ASMFLAGS = -f elf64

OBJS = boot_entry.o multiboot.o
OUT = boot.o

all: $(OUT)

boot_entry.o: boot.asm
	$(ASM) $(ASMFLAGS) $< -o $@

multiboot.o: multiboot.asm
	$(ASM) $(ASMFLAGS) $< -o $@

$(OUT): $(OBJS)
	ld -r -o $(OUT) $(OBJS)

clean:
	rm -f $(OBJS) $(OUT)
