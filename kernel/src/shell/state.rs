pub struct ShellState {
    cwd: [u8; 256],
    cwd_len: usize,
}

impl ShellState {
    pub const fn new() -> Self {
        let mut cwd = [0u8; 256];
        cwd[0] = b'/';
        
        Self { cwd: cwd, cwd_len: 1 }
    }

    pub fn cwd(&self) -> &[u8] {
        &self.cwd[..self.cwd_len]
    }

    pub fn set_cwd(&mut self, path: &[u8]) -> bool {
        if path.is_empty() || path.len() > self.cwd.len() {
            return false;
        }

        self.cwd[..path.len()].copy_from_slice(&path);
        self.cwd_len = path.len();

        true
    }
}