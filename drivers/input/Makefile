.PHONY: all clean

SUBDIRS := $(wildcard */)
SUBDIR_OBJS := $(foreach d,$(SUBDIRS),$(d)$(patsubst %/,%.o,$(d)))

OUT = input.o

all: $(OUT)

$(OUT): $(SUBDIR_OBJS)
	ld -r -o $(OUT) $(SUBDIR_OBJS)

$(SUBDIR_OBJS):
	$(MAKE) -C $(dir $@)

clean:
	for dir in $(SUBDIRS); do $(MAKE) -C $$dir clean; done
	rm -f $(OUT)
