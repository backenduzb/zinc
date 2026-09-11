ENTRY(_start)

SECTIONS
{
    . = 0xffffffff80000000;

    .text : ALIGN(4K)
    {
        *(.text*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata*)
    }

    .limine_requests : ALIGN(8)
    {
        KEEP(*(.limine_requests))
    }

    .data : ALIGN(4K)
    {
        *(.data*)
    }

    .bss : ALIGN(4K)
    {
        *(COMMON)
        *(.bss*)
    }
}
