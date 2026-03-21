ASM = nasm
ASMFLAGS = -f elf64

SRCS = $(wildcard *.asm)
OBJS = $(SRCS:.asm=.o)
OUT = cpu.o

all: $(OUT)

%.o: %.asm
	$(ASM) $(ASMFLAGS) $< -o $@

$(OUT): $(OBJS)
	ld -r -o $(OUT) $(OBJS)

clean:
	rm -f $(OBJS) $(OUT)
