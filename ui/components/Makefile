CC = gcc
CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../include

LOCAL_SRCS = $(wildcard *.c)
LOCAL_OBJS = $(LOCAL_SRCS:.c=.o)
SUBDIRS := $(wildcard */)
SUBDIR_OBJS := $(foreach d,$(SUBDIRS),$(d)$(patsubst %/,%.o,$(d)))
OUT = components.o

all: $(OUT)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

$(OUT): $(LOCAL_OBJS) $(SUBDIR_OBJS)
	ld -r -o $(OUT) $(LOCAL_OBJS) $(SUBDIR_OBJS)

$(SUBDIR_OBJS):
	$(MAKE) -C $(dir $@)

clean:
	for dir in $(SUBDIRS); do $(MAKE) -C $$dir clean; done
	rm -f $(LOCAL_OBJS) $(OUT)
