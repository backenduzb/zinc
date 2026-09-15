CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../../include

SRCS = $(wildcard *.c)
OBJS = $(SRCS:.c=.o)
OUT = ./build/hello.o

all: $(OUT)

build:
	mkdir -p build
	
%.o: %.c build
	$(CC) $(CFLAGS) -c $< -o $@

$(OUT): $(OBJS)
	ld -r -o $(OUT) $(OBJS)

clean:
	rm -f $(OBJS) $(OUT)
