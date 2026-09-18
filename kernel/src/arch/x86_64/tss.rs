/// 64-bit Task State Segment with the exact Intel byte layout.
///
/// WARNING: a plain `#[repr(C)]` struct with `rsp: [u64; 3]` after a `u32`
/// does NOT match the hardware layout: Rust aligns the `u64` array to
/// offset 8, while the CPU reads RSP0 from bytes 4..12. That latent
/// misalignment went unnoticed while RSP0 was never used, but the first
/// Ring 3 -> Ring 0 interrupt triple-faults on it (#SS on delivery).
/// Hence the raw byte array with explicit little-endian accessors.
///
/// Layout (Intel SDM Vol. 3, TSS64):
/// ```text
/// 0..4    reserved
/// 4..28   RSP0/RSP1/RSP2
/// 28..36  reserved
/// 36..92   IST1..IST7
/// 92..104 reserved + I/O map base
/// ```
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
        // I/O permission bitmap base = segment size -> empty bitmap,
        // Ring 3 I/O denied without reading past the segment.
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
