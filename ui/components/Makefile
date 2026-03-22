CC = gcc
LD = ld

CFLAGS = -ffreestanding -m64 -mno-red-zone -fno-pie -fno-pic -fno-stack-protector -I../../include

.DEFAULT_GOAL := all

SRCS = initalizer.c utils.c drawers.c splash.c psf1.c
OBJS = $(SRCS:.c=.o)

FONT_PSF_SRC := ../../assets/default8x16.psf
FONT_PSF_LOCAL := font.psf
FONT_OBJ := psf1_font.o
EXTRA_OBJS :=

ifneq ("$(wildcard $(FONT_PSF_SRC))","")
EXTRA_OBJS += $(FONT_OBJ)

$(FONT_PSF_LOCAL): $(FONT_PSF_SRC)
	cp $< $@

$(FONT_OBJ): $(FONT_PSF_LOCAL)
	$(LD) -r -b binary -o $@ $<
endif

all: fb.o

fb.o: $(OBJS) $(EXTRA_OBJS)
	$(LD) -r -o fb.o $(OBJS) $(EXTRA_OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(FONT_OBJ) $(FONT_PSF_LOCAL) fb.o
