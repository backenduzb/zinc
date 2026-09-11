bits 64

section .text

global _start
global timer_interrupt_stub

extern kmain
extern timer_handler

_start:
    cli

    call kmain

.hang:
    hlt
    jmp .hang

timer_interrupt_stub:
    push rax
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    call timer_handler
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rax
    iretq