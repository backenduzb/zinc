#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuContext {
    pub rsp: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

impl CpuContext {
    pub const fn empty() -> Self {
        Self {
            rsp: 0,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
        }
    }
}


core::arch::global_asm!(r#"
.global switch_context
.type switch_context, @function

switch_context:
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp

    mov [rdi], rsp
    mov rsp, [rsi]

    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15

    ret
"#);

unsafe extern "C" {
    pub fn switch_context(old: *mut CpuContext, new: *const CpuContext);
}