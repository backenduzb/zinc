CC := gcc
LD := ld

CFLAGS := -m64 -ffreestanding -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I./include

SUBDIRS := tests
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

SUBDIR_OBJS := \((foreach d,\)(SUBDIRS),\((d)/\)(notdir $(d)).o)

OUT := ../../build/c_objs.o

.PHONY: all clean $(SUBDIRS)

all: \((SUBDIRS)\)(OUT)

$(SUBDIRS):
	\((MAKE) -C\)@

%.o: %.c
	\((CC)\)(CFLAGS) -c \(< -o\)@

\((OUT):\)(OBJS) $(SUBDIRS)
	mkdir -p \((dir\)(OUT))
	\((LD) -r -o\)@ \((OBJS)\)(SUBDIR_OBJS)

clean:
	rm -f \((OBJS)\)(OUT)
	for dir in \((SUBDIRS); do\)(MAKE) -C $$dir clean; done