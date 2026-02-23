# boot.s — Minimal 64-bit entry point via Limine
.global _start
.extern kernel_main
.section .text
.code64
_start:
    # Stack
    lea stack_top(%rip), %rsp
    and $~0xF, %rsp     # align 16 bytes


    # Call kernel_main
    call kernel_main

.hang:
    hlt
    jmp .hang

# ==============================
# Stack
# ==============================
.section .bss
.align 16
stack:
    .skip 4096*16
stack_top:
