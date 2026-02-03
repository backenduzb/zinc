.section .text
.code32

.globl isr33
.extern keyboard_handler
isr33:
    pusha
    push %ds
    push %es
    push %fs
    push %gs
    
    mov $0x18, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    cld
    call keyboard_handler
    pop %gs
    pop %fs
    pop %es
    pop %ds
    popa
    iret

.globl isr_default
isr_default:
    iret

.section .text
.code32

.globl isr_halt
isr_halt:
    cli
1:
    hlt
    jmp 1b
