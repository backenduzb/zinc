ARCH ?= i686-elf

ifeq ($(origin CC), environment)
undefine CC
endif
ifeq ($(origin LD), environment)
undefine LD
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
LD ?= $(ARCH)-ld
CFLAGS ?= -ffreestanding -O2 -Wall -Wextra -nostdlib -Ikernel/include
ASFLAGS ?=
else
ifneq ($(origin CC), command line)
CC := clang
endif
LD ?= ld.lld
CFLAGS ?= -target $(ARCH) -ffreestanding -O2 -Wall -Wextra -nostdlib -Ikernel/include
ASFLAGS ?= -target $(ARCH)
endif

ifeq ($(shell command -v $(CC) 2>/dev/null),)
$(error Compiler '$(CC)' not found. Install clang or $(ARCH)-gcc, or run make CC=...)
endif

SRC = \
	kernel/src/arch/x86/boot.s \
	kernel/src/arch/x86/idt_load.s \
	kernel/src/arch/x86/isr.s \
	kernel/src/arch/x86/idt.c \
	kernel/src/arch/x86/pic.c \
	kernel/src/arch/x86/io.c \
	kernel/src/kernel/main.c \
	kernel/src/utils/string.c \

OBJ = $(SRC:.c=.o)
OBJ := $(OBJ:.s=.o)

.PHONY: all clean

all: $(OBJ)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.s
	$(CC) $(ASFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ)
