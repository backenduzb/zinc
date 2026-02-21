![header](https://capsule-render.vercel.app/api?type=waving&color=0:1a1b27,100:2483b6&height=200&section=header&text=ZINC%20OS&fontSize=60&fontColor=70a5fd&animation=fadeIn)


# Zinc architecture
󰣇 ~/OS/zinc   main  !+ ❯ tree                                                                                                                                                        10:35
.
├── arch
│   ├── kernel
│   │   ├── grub
│   │   │   └── grub.cfg
│   │   ├── include
│   │   │   ├── commands
│   │   │   │   └── commands.h
│   │   │   └── kernel
│   │   │       ├── idt.h
│   │   │       ├── io.h
│   │   │       ├── keyboard
│   │   │       │   ├── keyboard.h
│   │   │       │   └── keyreader.h
│   │   │       ├── pic.h
│   │   │       ├── string
│   │   │       │   └── str.h
│   │   │       ├── types.h
│   │   │       └── vga
│   │   │           ├── colors.h
│   │   │           ├── cursor.h
│   │   │           └── vga.h
│   │   └── src
│   │       ├── arch
│   │       │   └── x86
│   │       │       ├── boot.s
│   │       │       ├── idt.c
│   │       │       ├── idt_load.s
│   │       │       ├── io.c
│   │       │       ├── isr.s
│   │       │       └── pic.c
│   │       ├── kernel
│   │       │   └── main.c
│   │       ├── Makefile
│   │       └── utils
│   │           └── string.c
│   ├── linker.ld
│   ├── Makefile
│   └── qemu.log
├── commands
│   ├── commands.c
│   └── Makefile
├── docker-compose.yml
├── Dockerfile
├── drivers
│   ├── keyboard
│   │   ├── keyboard.c
│   │   └── keyreader.c
│   ├── Makefile
│   └── vga
│       ├── cursor.c
│       └── vga.c
├── LICENSE
├── Makefile
└── README.md

22 directories, 37 files

# Zinc OS ToDo
- [x] CPU controlling
- [x] Keyboard driver
- [x] VGA output
- [x] Create commmands 
- [ ] Timer / sleep_ms
- [ ] Command history
- [ ] Tab autocomplete