.PHONY: all tests

SUBDIRS := entry paging
SUBDIR_OBJS := $(foreach d,$(SUBDIRS),$(d)/$(notdir $(d)).o)

OUT := asmbuild.o

all: $(OUT)

tets:
	$(MAKE) -C entry

paging:
	$(MAKE) -C paging
	
$(SUBDIR_OBJS):
	$(MAKE) -C $(dir $@)

$(OUT): $(SUBDIR_OBJS)
	ld -r -o $(OUT) $(SUBDIR_OBJS)

clean:
	rm -f iso/boot/$(OUT)
	rm -f $(OUT) os.iso
	rm -f *.o
	for dir in $(SUBDIRS); do $(MAKE) -C $$dir clean; done
