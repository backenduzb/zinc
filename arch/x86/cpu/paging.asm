section .pgtables

align 4096

global pml4_table

pml4_table:
    dq pdpt_table + 0x003
    times 511 dq 0

align 4096
pdpt_table:
    dq pd_table0 + 0x003
    dq pd_table1 + 0x003
    dq pd_table2 + 0x003
    dq pd_table3 + 0x003
    times 508 dq 0

align 4096
pd_table0:
%assign i 0
%rep 512
    dq (0x00000000 + i * 0x200000) + 0x83
%assign i i+1
%endrep

align 4096
pd_table1:
%assign i 0
%rep 512
    dq (0x40000000 + i * 0x200000) + 0x83
%assign i i+1
%endrep

align 4096
pd_table2:
%assign i 0
%rep 512
    dq (0x80000000 + i * 0x200000) + 0x83
%assign i i+1
%endrep

align 4096
pd_table3:
%assign i 0
%rep 512
    dq (0xC0000000 + i * 0x200000) + 0x83
%assign i i+1
%endrep
