CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../include

SRCS = $(wildcard *.c)
OBJS = $(SRCS:.c=.o)

all: timer.o

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

timer.o: $(OBJS)
	ld -r -o timer.o $(OBJS)
	
clean:
	rm -f $(OBJS) timer.o
