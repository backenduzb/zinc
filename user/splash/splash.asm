BITS 64
DEFAULT REL

%define SYS_YIELD       0
%define SYS_EXIT        1
%define SYS_GFX_INFO    8
%define SYS_GFX_CLEAR   9
%define SYS_GFX_RECT    10

%define COLOR_BG        0x00100F18
%define COLOR_PANEL     0x00201D2B
%define COLOR_ACCENT    0x0000D9A6
%define COLOR_ACCENT2   0x0048F0C8

section .text.entry
global _start

_start:
    lea rdi, [rel gfx_info]
    mov rsi, 40
    mov rax, SYS_GFX_INFO
    int 0x80
    test rax, rax
    jnz .exit

    mov eax, [rel gfx_info + 8]
    mov [rel screen_w], eax

    mov eax, [rel gfx_info + 12]
    mov [rel screen_h], eax

    test eax, eax
    jz .exit

    mov eax, [rel screen_w]
    cmp eax, 300
    jb .exit

    mov eax, [rel screen_h]
    cmp eax, 180
    jb .exit

    mov rdi, COLOR_BG
    mov rax, SYS_GFX_CLEAR
    int 0x80
    test rax, rax
    jnz .exit

    mov eax, [rel screen_w]
    shr eax, 1
    sub eax, 150
    mov [rel panel_x], eax

    mov eax, [rel screen_h]
    shr eax, 1
    sub eax, 90
    mov [rel panel_y], eax

    mov eax, [rel panel_x]
    add eax, 20
    mov [rel rect_x], eax

    mov eax, [rel panel_y]
    add eax, 16
    mov [rel rect_y], eax

    mov dword [rel rect_w], 260
    mov dword [rel rect_h], 4
    mov dword [rel rect_color], COLOR_ACCENT
    call draw_rect
    test rax, rax
    jnz .exit

    mov eax, [rel panel_x]
    add eax, 20
    mov [rel rect_x], eax

    mov eax, [rel panel_y]
    add eax, 30
    mov [rel rect_y], eax

    mov dword [rel rect_w], 52
    mov dword [rel rect_h], 52
    mov dword [rel rect_color], COLOR_PANEL
    call draw_rect
    test rax, rax
    jnz .exit

    mov eax, [rel panel_x]
    add eax, 29
    mov [rel rect_x], eax

    mov eax, [rel panel_y]
    add eax, 39
    mov [rel rect_y], eax

    mov dword [rel rect_w], 34
    mov dword [rel rect_h], 5
    mov dword [rel rect_color], COLOR_ACCENT2
    call draw_rect
    test rax, rax
    jnz .exit

    mov eax, [rel panel_x]
    add eax, 29
    mov [rel rect_x], eax

    mov eax, [rel panel_y]
    add eax, 39
    mov [rel rect_y], eax

    mov dword [rel rect_w], 5
    mov dword [rel rect_h], 34
    mov dword [rel rect_color], COLOR_ACCENT
    call draw_rect
    test rax, rax
    jnz .exit

    mov eax, [rel panel_x]
    add eax, 29
    mov [rel rect_x], eax

    mov eax, [rel panel_y]
    add eax, 68
    mov [rel rect_y], eax

    mov dword [rel rect_w], 34
    mov dword [rel rect_h], 5
    mov dword [rel rect_color], COLOR_ACCENT2
    call draw_rect
    test rax, rax
    jnz .exit

    mov eax, [rel panel_x]
    add eax, 20
    mov [rel bar_x], eax

    mov eax, [rel panel_y]
    add eax, 102
    mov [rel bar_y], eax

    mov [rel rect_x], eax
    mov eax, [rel bar_x]
    mov [rel rect_x], eax

    mov eax, [rel bar_y]
    mov [rel rect_y], eax

    mov dword [rel rect_w], 260
    mov dword [rel rect_h], 3
    mov dword [rel rect_color], COLOR_PANEL
    call draw_rect
    test rax, rax
    jnz .exit

    mov dword [rel progress], 4

.progress:
    mov eax, [rel progress]
    cmp eax, 260
    jae .finish

    mov eax, [rel bar_x]
    mov [rel rect_x], eax

    mov eax, [rel bar_y]
    mov [rel rect_y], eax

    mov eax, [rel progress]
    mov [rel rect_w], eax

    mov dword [rel rect_h], 3
    mov dword [rel rect_color], COLOR_ACCENT

    call draw_rect
    test rax, rax
    jnz .exit

    mov ecx, 12000
.delay:
    pause
    loop .delay

    mov rax, SYS_YIELD
    int 0x80

    add dword [rel progress], 4
    jmp .progress

.finish:
    mov eax, [rel bar_x]
    mov [rel rect_x], eax

    mov eax, [rel bar_y]
    mov [rel rect_y], eax

    mov dword [rel rect_w], 260
    mov dword [rel rect_h], 3
    mov dword [rel rect_color], COLOR_ACCENT2

    call draw_rect

    mov ecx, 200000
.finish_delay:
    pause
    loop .finish_delay

.exit:
    mov rax, SYS_EXIT
    xor rdi, rdi
    int 0x80

.hang:
    pause
    jmp .hang

draw_rect:
    lea rdi, [rel rect]
    mov rsi, 20
    mov rax, SYS_GFX_RECT
    int 0x80
    ret

section .data

gfx_info:
    times 40 db 0

rect:
rect_x:     dd 0
rect_y:     dd 0
rect_w:     dd 0
rect_h:     dd 0
rect_color: dd 0

screen_w:   dd 0
screen_h:   dd 0
panel_x:    dd 0
panel_y:    dd 0
bar_x:      dd 0
bar_y:      dd 0
progress:   dd 0
