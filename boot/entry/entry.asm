BITS 64

section .text

global _start
global timer_interrupt_stub
global lapic_timer_interrupt_stub
global gp_fault_stub
global page_fault_stub
global double_fault_stub
global ud_fault_stub
global syscall_stub
global keyboard_interrupt_stub
global reschedule_interrupt_stub

extern kmain
extern timer_handler
extern lapic_timer_handler
extern gp_fault_handler
extern page_fault_handler
extern double_fault_handler
extern ud_fault_handler
extern syscall_handler
extern keyboard_handler
extern reschedule_handler

_start:
    cli

    call kmain

.start_hang:
    hlt
    jmp .start_hang

keyboard_interrupt_stub:
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
    call keyboard_handler

    test rdx, rdx
    jnz keyboard_fresh_thread

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

keyboard_fresh_thread:
    mov rdi, rax
    mov rsp, rax
    cld
    jmp rdx

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
    mov rdi, rax
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
    mov rdi, rax
    mov rsp, rax
    cld
    jmp rdx

reschedule_interrupt_stub:
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
    call reschedule_handler

    test rdx, rdx
    jnz reschedule_fresh_thread

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


reschedule_fresh_thread:
    mov rdi, rax
    mov rsp, rax
    cld
    jmp rdx

ud_fault_stub:
    mov rdi, [rsp]
    mov rsi, [rsp+8]
    call ud_fault_handler


ud_hang:
    hlt
    jmp ud_hang

gp_fault_stub:
    pop rdi
    mov rsi, [rsp]
    mov rdx, [rsp+8]
    call gp_fault_handler

gp_hang:
    hlt
    jmp gp_hang


page_fault_stub:
    pop rdi
    mov rsi, [rsp]
    mov rdx, [rsp+8]
    call page_fault_handler

pf_hang:
    hlt
    jmp pf_hang


double_fault_stub:
    pop rdi
    mov rsi, [rsp]
    mov rdx, [rsp+8]
    call double_fault_handler

df_hang:
    hlt
    jmp df_hang

syscall_stub:
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
    call syscall_handler

    mov rsp, rax

    test rdx, rdx
    jnz syscall_fresh_thread

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

syscall_fresh_thread:
    mov rdi, rax
    cld
    jmp rdx
