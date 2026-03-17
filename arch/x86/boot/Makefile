ASM = nasm
ASMFLAGS = -f elf64

OBJS = boot.o multiboot.o

all: $(OBJS)

%.o: %.asm
	$(ASM) $(ASMFLAGS) $< -o $@

clean:
	rm -f $(OBJS)
