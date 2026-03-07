//! Connection credentials.

use super::{Pid, Uid};

/// Credentials of a peer connection.
#[derive(Debug)]
pub struct Credentials {
    unix_user_id: Uid,
    process_id: Pid,
    #[cfg(target_os = "linux")]
    process_fd: std::os::fd::OwnedFd,
}

impl Credentials {
    /// Create new credentials for a peer connection.
    ///
    /// # Arguments
    /// * `unix_user_id` - The numeric Unix user ID.
    /// * `process_id` - The numeric process ID.
    /// * `process_fd` (Linux only) - A file descriptor pinning the process.
    pub(crate) fn new(
        unix_user_id: Uid,
        process_id: Pid,
        #[cfg(target_os = "linux")] process_fd: std::os::fd::OwnedFd,
    ) -> Self {
        Self {
            unix_user_id,
            process_id,
            #[cfg(target_os = "linux")]
            process_fd,
        }
    }

    /// The numeric Unix user ID, as defined by POSIX.
    pub fn unix_user_id(&self) -> Uid {
        self.unix_user_id
    }

    /// The numeric process ID, on platforms that have this concept.
    ///
    /// On Unix, this is the process ID defined by POSIX.
    pub fn process_id(&self) -> Pid {
        self.process_id
    }

    /// A file descriptor pinning the process, on platforms that have this concept.
    ///
    /// On Linux, the SO_PEERPIDFD socket option is a suitable implementation. This is safer to use
    /// to identify a process than the ProcessID, as the latter is subject to re-use attacks, while
    /// the FD cannot be recycled. If the original process no longer exists the FD will no longer
    /// be resolvable.
    #[cfg(target_os = "linux")]
    pub fn process_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;

        self.process_fd.as_fd()
    }
}

impl core::hash::Hash for Credentials {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.unix_user_id.hash(state);
        self.process_id.hash(state);
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;

            let fd = self.process_fd.as_raw_fd();
            fd.hash(state);
        }
    }
}
