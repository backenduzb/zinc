CC = gcc
LD = ld

CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../include

SRCS = string.c
OBJS = $(SRCS:.c=.o)

all: utils.o

utils.o: $(OBJS)
	$(LD) -r -o utils.o $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) utils.o
