
.global _start
.extern kernel_main
.section .text
.code64
_start:
    # Stack
    lea stack_top(%rip), %rsp
    and $~0xF, %rsp     


    call kernel_main

.hang:
    hlt
    jmp .hang

.section .bss
.align 16
stack:
    .skip 4096*16
stack_top:
