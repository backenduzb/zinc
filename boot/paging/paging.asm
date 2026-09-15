global switch_address_space
global enter_new_address_space

section .text

switch_address_space:
    ; RDI = PML4 physical address
    mov cr3, rdi
    ret


enter_new_address_space:
    ; RDI = PML4 physical address
    ; RSI = new stack top
    ; RDX = entry address

    cli

    mov cr3, rdi
    mov rsp, rsi

    jmp rdx