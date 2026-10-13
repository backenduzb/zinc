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

        if path[0] != b'/' {
            let cwd = self.cwd();
            out[..cwd.len()].copy_from_slice(cwd);
            len = cwd.len();
        }

        let mut index = 0;

        while index < path.len() {
            while index < path.len() && path[index] == b'/' {
                index += 1;
            }

            if index >= path.len() {
                break;
            }

            let start = index;

            while index < path.len() && path[index] != b'/' {
                index += 1;
            }

            let component = &path[start..index];

            if component == b"." {
                continue;
            }

            if component == b".." {
                if len > 1 {
                    while len > 1 && out[len - 1] != b'/' {
                        len -= 1;
                    }

                    if len > 1 {
                        len -= 1;
                    }
                }

                continue;
            }

            if len > 1 {
                if len >= out.len() {
                    return None;
                }

                out[len] = b'/';
                len += 1;
            }

            if len + component.len() > out.len() {
                return None;
            }

            out[len..len + component.len()].copy_from_slice(component);
            len += component.len();
        }

        if len == 0 {
            out[0] = b'/';
            len = 1;
        }

        Some(len)
    }
}
