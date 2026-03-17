ASM = nasm
ASMFLAGS = -f elf64

OBJ = gdt.o paging.o idt.o isr.o

all: $(OBJ)

%.o: %.asm
	$(ASM) $(ASMFLAGS) $< -o $@

clean:
	rm -f *.o
