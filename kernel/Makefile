CC = x86_64-elf-gcc

CFLAGS = -O2 -ffreestanding -fno-pie -no-pie -mno-red-zone -mcmodel=kernel -I../include -mno-sse -mno-mmx -mno-avx -mgeneral-regs-only

SRC = main.c\
	  video/framebuffer.c\
	  interrupts/idt.c\
	  interrupts/pic.c\
	  interrupts/interrupts.c
OBJ = $(SRC:.c=.o)

all: $(OBJ)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ)
	rm -f interrupts/*.o
