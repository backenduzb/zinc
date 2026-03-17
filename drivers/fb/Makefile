CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -I../../include

all: fb.o

fb.o: initalizer.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f main.o
