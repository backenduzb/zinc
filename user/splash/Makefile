
CARGO := cargo +nightly
TARGET := x86_64-unknown-none
OUT := target/$(TARGET)/release/zinc-splash
ELF := splash.elf

.PHONY: all clean inspect

all: $(ELF)

$(ELF): Cargo.toml linker.ld .cargo/config.toml $(wildcard src/*.rs)
	$(CARGO) build -Z build-std=core --release
	cp $(OUT) $(ELF)

inspect: $(ELF)
	readelf -h -l $(ELF)
	nm -n $(ELF) | head -30

clean:
	$(CARGO) clean
	rm -f $(ELF)
