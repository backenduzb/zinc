#[repr(C, align(8))]
pub struct TaskStateSegment {
    data: [u8; 104],
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self { data: Self::blank() }
    }

    const fn blank() -> [u8; 104] {
        let mut data = [0u8; 104];
        data[102] = 104;
        data[103] = 0;
        data
    }

    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.data[4..12].copy_from_slice(&stack_top.to_le_bytes());
    }

    pub fn kernel_stack(&self) -> u64 {
        u64::from_le_bytes([
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
            self.data[8],
            self.data[9],
            self.data[10],
            self.data[11],
        ])
    }
}
