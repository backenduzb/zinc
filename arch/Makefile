AS = x86_64-elf-as

SRC = x86_64/boot.s
OBJ = x86_64/boot.o

all: $(OBJ)

$(OBJ): $(SRC)
	$(AS) $(SRC) -o $(OBJ)

clean:
	rm -f $(OBJ)