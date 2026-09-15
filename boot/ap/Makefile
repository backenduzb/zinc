ASM = nasm

BINFLAGS = -f bin

TRAMPOLINE_SRC = trampoline.asm
TRAMPOLINE_BIN = build/trampoline.bin

all: $(TRAMPOLINE_BIN)

build:
	mkdir -p build

$(TRAMPOLINE_BIN): $(TRAMPOLINE_SRC) | build
	$(ASM) $(BINFLAGS) $< -o $@

clean:
	rm -rf build