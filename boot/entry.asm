bits 64

section .text

global _start
global timer_interrupt_stub
global gp_fault_stub
global page_fault_stub
global double_fault_stub

extern kmain
extern timer_handler
extern gp_fault_handler
extern page_fault_handler
extern double_fault_handler

_start:
    cli

    call kmain

.hang:
    hlt
    jmp .hang

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

    ; rax = rsp (resume frame) yoki stack_top (fresh thread)
    ; rdx = 0 (resume: pops+iretq) yoki entry (fresh: jmp)
    mov rsp, rax
    test rdx, rdx
    jnz .fresh_thread

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

.fresh_thread:
    ; Fresh thread: interrupt o'chiq holatda (IF=0),
    ; to'g'ridan-to'g'ri jmp. Thread o'zi sti qiladi.
    ; (rdx = entry, rsp = stack_top)
    cld
    jmp rdx

; #GP (13), #PF (14), #DF (8) CPU error code push qiladi.
; Uni pop qilib handler'ga birinchi argument sifatida uzatamiz.
gp_fault_stub:
    pop rdi
    call gp_fault_handler
.hang_gp:
    hlt
    jmp .hang_gp

page_fault_stub:
    pop rdi
    call page_fault_handler
.hang_pf:
    hlt
    jmp .hang_pf

double_fault_stub:
    pop rdi
    call double_fault_handler
.hang_df:
    hlt
    jmp .hang_df