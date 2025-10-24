use std::{
    ffi::OsString,
    io,
    mem::{MaybeUninit, transmute},
    os::{fd::RawFd, raw::c_void},
};

use bytes::BufMut;
use libc::{pid_t, seccomp_notif};
use tokio::io::ReadBuf;

pub trait FromSyscallArg: Sized {
    fn from_syscall_arg(arg: u64) -> io::Result<Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct CStrPtr {
    remote_ptr: *mut c_void,
}

/// Represents the caller of a syscall. Needed to read memory from the caller's address space.
#[derive(Debug, Clone, Copy)]
pub struct Caller<'a> {
    pid: pid_t,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Caller<'a> {
    /// Creates a `Caller` for the given pid with a local lifetime.
    #[doc(hidden)] // only exposed for `impl_handler` macro
    pub fn with_pid<R, F: FnOnce(Caller<'_>) -> R>(pid: pid_t, f: F) -> R {
        f(Self { pid, _marker: std::marker::PhantomData })
    }
}

impl CStrPtr {
    // Reads the C string from the remote process into the provided buffer.
    // Returns whether the read was successful or not because the buffer was filled before a null-terminator was found.
    pub fn read<B: BufMut>(&self, caller: Caller<'_>, buf: &mut B) -> io::Result<bool> {
        loop {
            let chunk = buf.chunk_mut();
            if chunk.len() == 0 {
                return Ok(false);
            }

            let local_iov =
                libc::iovec { iov_base: chunk.as_mut_ptr().cast(), iov_len: chunk.len() };

            let remote_iov = libc::iovec { iov_base: self.remote_ptr, iov_len: chunk.len() };

            let read_size =
                unsafe { libc::process_vm_readv(caller.pid, &local_iov, 1, &remote_iov, 1, 0) };

            let Ok(read_size) = usize::try_from(read_size) else {
                return Err(io::Error::last_os_error());
            };

            // chunk[..read_size] are all initialized, but we are only going to advance until '\0'
            let chunk = unsafe {
                transmute::<&[MaybeUninit<u8>], &[u8]>(&chunk.as_uninit_slice_mut()[..read_size])
            };
            let Some(nul_index) = chunk.iter().position(|byte| *byte == b'\0') else {
                // No '\0' found, could be a partial read, advance all of `read_size` and continue reading.
                unsafe { buf.advance_mut(read_size) };
                continue;
            };
            unsafe { buf.advance_mut(nul_index) };
            return Ok(true);
        }
    }

    // Reads the C string from the remote process into a fixed-size buffer.
    // The closure is called with `Some(&[u8])` if a null-terminator was found within the buffer size,
    // or `None` if the buffer was filled without encountering a null-terminator.
    pub fn read_with_buf<const BUF_SIZE: usize, R, F: FnOnce(Option<&[u8]>) -> io::Result<R>>(
        &self,
        caller: Caller<'_>,
        f: F,
    ) -> io::Result<R> {
        let mut read_buf: [MaybeUninit<u8>; BUF_SIZE] = [const { MaybeUninit::uninit() }; BUF_SIZE];
        let mut read_buf = ReadBuf::uninit(read_buf.as_mut_slice());
        let success = self.read(caller, &mut read_buf)?;
        f(if success { Some(read_buf.filled()) } else { None })
    }
}

impl FromSyscallArg for CStrPtr {
    fn from_syscall_arg(arg: u64) -> io::Result<Self> {
        Ok(Self { remote_ptr: arg as _ })
    }
}

#[derive(Debug)]
pub struct Ignored(());
impl FromSyscallArg for Ignored {
    fn from_syscall_arg(_arg: u64) -> io::Result<Self> {
        Ok(Ignored(()))
    }
}

#[derive(Debug)]
pub struct Fd {
    fd: RawFd,
}
impl FromSyscallArg for Fd {
    fn from_syscall_arg(arg: u64) -> io::Result<Self> {
        Ok(Self { fd: arg as _ })
    }
}

impl Fd {
    // TODO: allocate in arena
    pub fn get_path(&self, caller: Caller<'_>) -> nix::Result<OsString> {
        nix::fcntl::readlink(
            if self.fd == libc::AT_FDCWD {
                format!("/proc/{}/cwd", caller.pid)
            } else {
                format!("/proc/{}/fd/{}", caller.pid, self.fd)
            }
            .as_str(),
        )
    }
}

pub trait FromNotify: Sized {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self>;
}

impl<T: FromSyscallArg> FromNotify for (T,) {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((T::from_syscall_arg(notif.data.args[0])?,))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg> FromNotify for (T1, T2) {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((T1::from_syscall_arg(notif.data.args[0])?, T2::from_syscall_arg(notif.data.args[1])?))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg, T3: FromSyscallArg> FromNotify for (T1, T2, T3) {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((
            T1::from_syscall_arg(notif.data.args[0])?,
            T2::from_syscall_arg(notif.data.args[1])?,
            T3::from_syscall_arg(notif.data.args[2])?,
        ))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg, T3: FromSyscallArg, T4: FromSyscallArg> FromNotify
    for (T1, T2, T3, T4)
{
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((
            T1::from_syscall_arg(notif.data.args[0])?,
            T2::from_syscall_arg(notif.data.args[1])?,
            T3::from_syscall_arg(notif.data.args[2])?,
            T4::from_syscall_arg(notif.data.args[3])?,
        ))
    }
}
