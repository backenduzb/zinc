
ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = 0x400000;

    .text : ALIGN(0x1000)
    {
        KEEP(*(.text.entry))
        *(.text*)
    } :text

    .rodata : ALIGN(0x1000)
    {
        *(.rodata*)
    } :text

    . = ALIGN(0x1000);

    .data :
    {
        *(.data*)
        *(.got*)
    } :data

    .bss :
    {
        *(COMMON)
        *(.bss*)
    } :data

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
