use std::{
    ffi::{OsString, c_int},
    io::{self, IoSliceMut, Read},
    marker::PhantomData,
    mem::MaybeUninit,
    os::{fd::RawFd, raw::c_void},
};

use libc::{pid_t, seccomp_notif};
use nix::sys::uio::{RemoteIoVec, process_vm_readv};

pub trait FromSyscallArg: Sized {
    fn from_syscall_arg(arg: u64) -> io::Result<Self>;
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

    pub fn read_vm(self, starting_addr: usize) -> ProcessVmReader<'a> {
        ProcessVmReader { caller: self, current_addr: starting_addr }
    }
}

pub struct ProcessVmReader<'a> {
    caller: Caller<'a>,
    current_addr: usize,
}

impl<'a> io::Read for ProcessVmReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buf_len = buf.len();
        let read_len = process_vm_readv(
            nix::unistd::Pid::from_raw(self.caller.pid),
            &mut [IoSliceMut::new(buf)],
            &[RemoteIoVec { base: self.current_addr, len: buf_len }],
        )?;
        self.current_addr = self.current_addr.checked_add(read_len).ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "address overflow while reading remote process")
        })?;
        Ok(read_len)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CStrPtr {
    remote_ptr: usize,
}

impl CStrPtr {
    // Reads the C string from the remote process into the provided buffer.
    // Returns:
    /// - `Ok(Some(n))` if a null-terminator was found at position n of the buffer,
    /// - `Ok(None)` if the buffer was filled without encountering a null-terminator.
    /// - `Err(UnexpectedEof)` if Eof was reached without encountering a null-terminator.
    /// - `Err(other_err)` on other errors from reading the remote process memory.
    pub fn read(&self, caller: Caller<'_>, buf: &mut [u8]) -> io::Result<Option<usize>> {
        let mut reader = caller.read_vm(self.remote_ptr);
        let mut pos = 0;
        while let Some((_, unfilled)) = buf.split_at_mut_checked(pos) {
            if unfilled.is_empty() {
                break;
            }
            let read_bytes = reader.read(unfilled)?;
            if read_bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "reached EOF while reading C string from remote process",
                ));
            }
            if let Some(null_pos) = unfilled[..read_bytes].iter().position(|&b| b == 0) {
                return Ok(Some(pos + null_pos));
            }
            pos += read_bytes;
        }
        Ok(None)
    }
}

impl FromSyscallArg for CStrPtr {
    fn from_syscall_arg(arg: u64) -> io::Result<Self> {
        Ok(Self { remote_ptr: arg as _ })
    }
}

pub struct Ptr<T> {
    remote_ptr: *mut c_void,
    _marker: PhantomData<T>,
}
impl<T> FromSyscallArg for Ptr<T> {
    fn from_syscall_arg(arg: u64) -> io::Result<Self> {
        Ok(Self { remote_ptr: arg as _, _marker: PhantomData })
    }
}
impl<T> Ptr<T> {
    /// Reads the value of type T from the remote process memory.
    /// # Safety:
    /// The remote pointer must be valid and point to a value of type T in the remote process memory.
    pub unsafe fn read(&self, caller: Caller<'_>) -> io::Result<T> {
        let mut reader = caller.read_vm(self.remote_ptr as usize);
        let mut buf = MaybeUninit::<T>::zeroed();
        let buf_slice = unsafe {
            std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, std::mem::size_of::<T>())
        };
        reader.read_exact(buf_slice)?;
        Ok(unsafe { buf.assume_init() })
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

impl Fd {
    pub fn cwd() -> Self {
        Self { fd: libc::AT_FDCWD }
    }
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

impl FromSyscallArg for c_int {
    fn from_syscall_arg(arg: u64) -> io::Result<Self> {
        Ok(arg as _)
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
