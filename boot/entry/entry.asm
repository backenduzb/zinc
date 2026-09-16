BITS 64

section .text

global _start
global timer_interrupt_stub
global lapic_timer_interrupt_stub
global gp_fault_stub
global page_fault_stub
global double_fault_stub

extern kmain
extern timer_handler
extern lapic_timer_handler
extern gp_fault_handler
extern page_fault_handler
extern double_fault_handler


_start:
    cli

    call kmain

.start_hang:
    hlt
    jmp .start_hang


timer_interrupt_stub:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    cld

    mov rdi, rsp
    call timer_handler

    mov rsp, rax

    test rdx, rdx
    jnz timer_fresh_thread

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    iretq


timer_fresh_thread:
    cld
    jmp rdx


lapic_timer_interrupt_stub:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    cld

    mov rdi, rsp
    call lapic_timer_handler

    test rdx, rdx
    jnz lapic_timer_fresh_thread

    mov rsp, rax

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    iretq


lapic_timer_fresh_thread:

    mov rsp, rax
    cld
    jmp rdx
    
gp_fault_stub:
    pop rdi
    call gp_fault_handler

gp_hang:
    hlt
    jmp gp_hang


; ============================================================
; Page Fault
; ============================================================

page_fault_stub:
    pop rdi
    call page_fault_handler

pf_hang:
    hlt
    jmp pf_hang


; ============================================================
; Double Fault
; ============================================================

double_fault_stub:
    pop rdi
    call double_fault_handler

df_hang:
    hlt
    jmp df_hang