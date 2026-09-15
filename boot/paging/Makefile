ASM = nasm
ASMFLAGS = -f elf64

SRCS = $(wildcard *.asm)
OBJS = $(SRCS:.asm=.o)
OUT = ./build/boot.o

all: $(OUT)

build:
	mkdir -p build
	
%.o: %.asm build
	$(ASM) $(ASMFLAGS) $< -o $@

$(OUT): $(OBJS)
	ld -r -o $(OUT) $(OBJS)

clean:
	rm -f $(OBJS) $(OUT)
