CC := gcc
LD := ld

CFLAGS := -m64 -ffreestanding -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../include

SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

OUT := tests.o

.PHONY: all clean

all: $(OUT)

%.o: %.c
	\((CC)\)(CFLAGS) -c \(< -o\)@

\((OUT):\)(OBJS)
	\((LD) -r -o\)@ $(OBJS)

clean:
	rm -f \((OBJS)\)(OUT)