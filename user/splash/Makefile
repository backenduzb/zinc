
ASM := nasm
LD := /usr/bin/ld
OUT := splash.elf

.PHONY: all clean

all: $(OUT)

splash.o: splash.asm
	$(ASM) -f elf64 $< -o $@


$(OUT): splash.o linker.ld
	$(LD) -nostdlib -static -z max-page-size=0x1000 -T linker.ld -o $@ splash.o

clean:
	rm -f splash.o $(OUT)
