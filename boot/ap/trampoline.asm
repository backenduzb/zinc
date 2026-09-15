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

    ; Load BSP page table
    mov eax, [abs trampoline_cr3]
    mov cr3, eax

    ; Enable Long Mode
    mov ecx, 0xC0000080
    rdmsr

    or eax, (1 << 8)

    wrmsr

    ; Enable paging
    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    ; Enter 64-bit code segment
    jmp 0x18:long_mode


BITS 64

long_mode:

    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Load AP stack
    mov rsp, [abs trampoline_stack]

    ; 16-byte alignment
    and rsp, -16

    ; Load AP entry point
    mov rax, [abs trampoline_entry]

    ; Enter Rust
    call rax


.hang:
    cli
    hlt
    jmp .hang


; -------------------------------------------------
; Trampoline GDT
; -------------------------------------------------

align 8

trampoline_gdt:

    ; 0x00: Null
    dq 0x0000000000000000

    ; 0x08: 32-bit code
    dq 0x00CF9A000000FFFF

    ; 0x10: 32-bit data
    dq 0x00CF92000000FFFF

    ; 0x18: 64-bit code
    dq 0x00AF9A000000FFFF

    ; 0x20: 64-bit data
    dq 0x00AF92000000FFFF

trampoline_gdt_end:


; -------------------------------------------------
; GDT descriptor
; -------------------------------------------------

trampoline_gdt_descriptor:

    dw trampoline_gdt_end - trampoline_gdt - 1
    dd trampoline_gdt


; -------------------------------------------------
; Fixed parameter area
;
; Physical address:
;
; 0x8000 + 0x100 = 0x8100 -> CR3
; 0x8000 + 0x108 = 0x8108 -> STACK
; 0x8000 + 0x110 = 0x8110 -> ENTRY
; -------------------------------------------------

times 0x100 - ($ - $$) db 0


; 0x100
trampoline_cr3:
    dd 0
    dd 0


; 0x108
trampoline_stack:
    dq 0


; 0x110
trampoline_entry:
    dq 0


; -------------------------------------------------
; End
; -------------------------------------------------


times 0x120 - ($ - $$) db 0

ap_trampoline_end:

