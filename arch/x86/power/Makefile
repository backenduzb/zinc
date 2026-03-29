CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../../include

SRCS = $(wildcard *.c)
OBJS = $(SRCS:.c=.o)
OUT = interrupts.o

all: $(OUT)

$(OUT): $(OBJS)
    ld -r -o $(OUT) $(OBJS)
    
%.o: %.c
    $(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(OUT)
