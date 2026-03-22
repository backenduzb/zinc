CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../include

SRCS = $(wildcard *.c)
OBJ_SUFFIX = _obj.o
OBJS = $(SRCS:.c=$(OBJ_SUFFIX))

OUT = acpi.o

all: $(OUT)

%$(OBJ_SUFFIX): %.c
	$(CC) $(CFLAGS) -c $< -o $@

$(OUT): $(OBJS)
	ld -r -o $(OUT) $(OBJS)

clean:
	rm -f $(OBJS) $(OUT)
