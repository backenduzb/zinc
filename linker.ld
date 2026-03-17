ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
    . = 1M;

    .text : {
        *(.multiboot_header)
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .pgtables : ALIGN(4096) {
        *(.pgtables)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
    }
}
