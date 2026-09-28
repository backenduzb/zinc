pub struct ShellState {
    cwd: [u8; 256],
    cwd_len: usize,
}

impl ShellState {
    pub const fn new() -> Self {
        let mut cwd = [0u8; 256];
        cwd[0] = b'/';

        Self {
            cwd: cwd,
            cwd_len: 1,
        }
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

    pub fn resolve_path(&self, path: &[u8], out: &mut [u8; 256]) -> Option<usize> {
        if path.is_empty() {
            return None;
        }

        let mut len = 0;

        if path[0] == b'/' {
            if path.len() > out.len() {
                return None;
            }

            out[..path.len()].copy_from_slice(path);
            return Some(path.len());
        }

        let cwd = self.cwd();

        if cwd.len() + 1 + path.len() > out.len() {
            return None;
        }

        out[..cwd.len()].copy_from_slice(cwd);
        len = cwd.len();

        if len > 1 {
            out[len] = b'/';
            len += 1;
        }

        out[len..len + path.len()].copy_from_slice(path);
        len += path.len();

        Some(len)
    }
}
