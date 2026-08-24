use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

pub(crate) struct FileLock {
    file: File,
}

impl FileLock {
    pub(crate) fn acquire(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;

        loop {
            // SAFETY: `file` owns this valid descriptor for the lifetime of the lock.
            let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
            if result == 0 {
                return Ok(Self { file });
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: `self.file` remains open until after `drop` returns.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
