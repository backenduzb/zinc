CC = gcc
LD = ld

CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../include

SRCS = initalizer.c utils.c drawers.c splash.c
OBJS = $(SRCS:.c=.o)

all: fb.o

fb.o: $(OBJS)
	$(LD) -r -o fb.o $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) fb.o
