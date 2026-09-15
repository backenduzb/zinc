#[repr(C, packed)]
pub struct TaskStateSegment {
    _reserved0: u32,

    pub rsp: [u64; 3],

    _reserved1: u64,

    pub ist: [u64; 7],

    _reserved2: u64,
    _reserved3: u16,

    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            _reserved0: 0,
            rsp: [0; 3],
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            iomap_base: core::mem::size_of::<Self>() as u16,
        }
    }

    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.rsp[0] = stack_top;
    }
}