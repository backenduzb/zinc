section .text
bits 32
global _start
extern kernel_main

extern gdt64_ptr
extern CODE_SEG
extern DATA_SEG
extern pml4_table

_start:
    cli
    mov esp, stack_top32
    mov [mbi_magic], eax
    mov [mbi_ptr], ebx

    lgdt [gdt64_ptr]

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov eax, pml4_table
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    jmp CODE_SEG:long_mode_start

bits 64
long_mode_start:

    mov ax, DATA_SEG
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    mov rsp, stack_top64
    mov edi, [mbi_magic]
    mov esi, [mbi_ptr]

    call kernel_main

.hang:
    hlt
    jmp .hang

section .bss
align 16
stack_bottom32:
    resb 4096
stack_top32:

align 16
stack_bottom64:
    resb 16384
stack_top64:

mbi_magic:
    resd 1
mbi_ptr:
    resd 1
