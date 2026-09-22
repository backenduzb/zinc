use crate::filesystem::vfs::VfsFile;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    File(VfsFile),
}

pub struct FdTable {
    entries: [Option<FileDescriptor>; 32],
}

impl FdTable {
    pub const fn new() -> Self {
        let mut entries = [const {None}; 32];

        entries[STDIN] = Some(FileDescriptor::Stdin);
        entries[STDOUT] = Some(FileDescriptor::Stdout);
        entries[STDERR] = Some(FileDescriptor::Stderr);

        Self { entries }
    }

    pub fn alloc(&mut self, file: VfsFile) -> Option<usize> {
        self.alloc_descriptor(FileDescriptor::File(file))
    }

    fn alloc_descriptor(&mut self, descriptor: FileDescriptor) -> Option<usize> {
        for fd in 3..self.entries.len() {
            if self.entries[fd].is_none() {
                self.entries[fd] = Some(descriptor);
                return Some(fd);
            }
        }

        None
    }

    pub fn get(&self, fd: usize) -> Option<&FileDescriptor> {
        self.entries.get(fd)?.as_ref()
    }

    pub fn get_mut(&mut self, fd: usize) -> Option<&mut FileDescriptor> {
        self.entries.get_mut(fd)?.as_mut()
    }

    pub fn close(&mut self, fd: usize) -> Option<FileDescriptor> {
        if fd < 3 {
            return None;
        }

        self.entries.get_mut(fd)?.take()
    }

    pub fn is_open(&self, fd: usize) -> bool {
        self.entries
            .get(fd)
            .is_some_and(|entry| entry.is_some())
    }

    pub fn stdin(&self) -> bool {
        matches!(self.get(STDIN), Some(FileDescriptor::Stdin))
    }

    pub fn stdout(&self) -> bool {
        matches!(self.get(STDOUT), Some(FileDescriptor::Stdout))
    }

    pub fn stderr(&self) -> bool {
        matches!(self.get(STDERR), Some(FileDescriptor::Stderr))
    }

    pub fn is_standard_output(&self, fd: usize) -> bool {
        matches!(
            self.get(fd), Some(FileDescriptor::Stdout) | Some(FileDescriptor::Stderr)
        )
    }
}
