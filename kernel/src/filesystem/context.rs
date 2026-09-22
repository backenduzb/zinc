use crate::drivers::storage::ide::IdeDevice;
use crate::filesystem::ext2::{parse_group_descriptor, parse_superblock};
use crate::filesystem::vfs::{Vfs, VfsFile};

pub struct FilesystemContext {
    vfs: Vfs,
}

impl FilesystemContext {
    pub fn new() -> Option<Self> {
        let mut ide = IdeDevice::new();

        let mut sb_buffer = [0u8; 512];
        ide.read_sector(2, &mut sb_buffer).ok()?;

        let sb = parse_superblock(&sb_buffer)?;

        let mut gd_buffer = [0u8; 512];
        ide.read_sector(4, &mut gd_buffer).ok()?;

        let gd = parse_group_descriptor(&gd_buffer)?;

        let vfs = Vfs::new(
            gd.inode_table_block(),
            256,
            sb,
        );

        Some(Self { vfs })
    }

    pub fn open(&self, path: &[u8]) -> Option<VfsFile> {
        let mut ide = IdeDevice::new();
        self.vfs.open(&mut ide, path)
    }

    pub fn read(
        &self,
        file: &mut VfsFile,
        buffer: &mut [u8],
    ) -> usize {
        let mut ide = IdeDevice::new();

        self.vfs.read(
            &mut ide,
            file,
            buffer,
        )
    }

    pub fn close(&self, file: VfsFile) {
        self.vfs.close(file);
    }
}

static mut FILESYSTEM_CONTEXT: Option<FilesystemContext> = None;

pub fn init() -> bool {
    let context = match FilesystemContext::new() {
        Some(context) => context,
        None => return false,
    };

    unsafe {
        FILESYSTEM_CONTEXT = Some(context);
    }

    true
}

#[inline]
pub fn filesystem() -> *const FilesystemContext {
    unsafe {
        match core::ptr::addr_of!(FILESYSTEM_CONTEXT).read() {
            Some(_) => {
                let option_ptr = core::ptr::addr_of!(FILESYSTEM_CONTEXT);

                (*option_ptr)
                    .as_ref()
                    .map(|context| context as *const FilesystemContext)
                    .unwrap_or(core::ptr::null())
            }

            None => {
                core::ptr::null()
            }
        }
    }
}