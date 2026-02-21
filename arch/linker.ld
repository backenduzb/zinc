ENTRY(_start)

SECTIONS {
    . = 1M;
    
    .multiboot : {
        KEEP(*(.multiboot))
    }
    
    .text : {
        *(.text*)
    }
    
    .rodata : {
        *(.data*)
    }
    
    .data : {
        *(.data*)
    }
    
    .bss : {
        *(COMMON)
        *(.bss*)
    }
}
