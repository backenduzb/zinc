section .data
align 8

global gdt64_ptr
global CODE_SEG
global DATA_SEG

gdt64:
    dq 0

gdt64_code:
    dq 0x00AF9A000000FFFF

gdt64_data:
    dq 0x00AF92000000FFFF

gdt64_end:

gdt64_ptr:
    dw gdt64_end - gdt64 - 1
    dq gdt64

CODE_SEG equ gdt64_code - gdt64
DATA_SEG equ gdt64_data - gdt64
