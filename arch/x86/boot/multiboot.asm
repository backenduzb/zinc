section .multiboot_header
align 8

header_start:

dd 0xe85250d6
dd 0
dd header_end - header_start
dd - (0xe85250d6 + 0 + (header_end - header_start))

dw 5
dw 0
dd 24
dd 0
dd 0
dd 0
dd 0

dw 0
dw 0
dd 8

header_end:
