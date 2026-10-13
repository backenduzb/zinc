BITS 16
ORG 0x8000

ap_trampoline_start:

    cli

    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    lgdt [abs trampoline_gdt_descriptor]

    mov eax, cr0
    or eax, 1
    mov cr0, eax

    jmp 0x08:protected_mode


BITS 32

protected_mode:

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Enable PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    mov eax, [abs trampoline_cr3]
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr

    or eax, (1 << 8) | (1 << 11)

    wrmsr

    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    jmp 0x18:long_mode


BITS 64

long_mode:

    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov ss, ax

    mov rsp, [abs trampoline_stack]
    and rsp, -16
    mov rax, [abs trampoline_entry]

    call rax


.hang:
    cli
    hlt
    jmp .hang


align 8

trampoline_gdt:

    dq 0x0000000000000000

    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
    dq 0x00AF9A000000FFFF

    dq 0x00AF92000000FFFF

trampoline_gdt_end:



trampoline_gdt_descriptor:

    dw trampoline_gdt_end - trampoline_gdt - 1
    dd trampoline_gdt



times 0x100 - ($ - $$) db 0


trampoline_cr3:
    dd 0
    dd 0


trampoline_stack:
    dq 0


trampoline_entry:
    dq 0


times 0x120 - ($ - $$) db 0

ap_trampoline_end:

