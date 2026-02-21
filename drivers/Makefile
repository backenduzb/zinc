ARCH ?= i686-elf

ifeq ($(origin CC), environment)
undefine CC
endif
ifeq ($(origin CFLAGS), environment)
undefine CFLAGS
endif
ifeq ($(origin ASFLAGS), environment)
undefine ASFLAGS
endif

ifneq ($(shell command -v $(ARCH)-gcc 2>/dev/null),)
ifneq ($(origin CC), command line)
CC := $(ARCH)-gcc
endif
CFLAGS ?= -ffreestanding -O2 -Wall -Wextra -nostdlib -I../arch/kernel/include
ASFLAGS ?=
else
ifneq ($(origin CC), command line)
CC := clang
endif
CFLAGS ?= -target $(ARCH) -ffreestanding -O2 -Wall -Wextra -nostdlib -I../arch/kernel/include
ASFLAGS ?= -target $(ARCH)
endif

ifeq ($(shell command -v $(CC) 2>/dev/null),)
$(error Compiler '$(CC)' not found. Install clang or $(ARCH)-gcc, or run make CC=...)
endif

SRC = \
	keyboard/keyboard.c \
	keyboard/keyreader.c \
	vga/vga.c \
	vga/cursor.c \
	time/btime.c \

OBJ = $(SRC:.c=.o)

.PHONY: all clean

all: $(OBJ)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ)
