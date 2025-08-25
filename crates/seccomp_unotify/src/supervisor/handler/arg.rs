use std::{
    ffi::{CStr, OsString}, io, mem::{transmute, MaybeUninit}, os::{fd::RawFd, raw::c_void}
};

use arrayvec::ArrayVec;
use bytes::BufMut;
use libc::{pid_t, seccomp_notif};
use tokio::io::ReadBuf;

pub trait FromSyscallArg: Sized {
    fn from_syscall_arg(pid: u32, arg: u64) -> io::Result<Self>;
}

#[derive(Debug)]
pub struct CStrPtr {
    pid: pid_t,
    remote_ptr: *mut c_void,
}

impl CStrPtr {
    pub fn read<B: BufMut>(&self, buf: &mut B) -> io::Result<()> {
        loop {
            let chunk = buf.chunk_mut();
            if chunk.len() == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "CStrPtr::read: buf is filled before null-terminator is found"
                ))
            }

            let local_iov = libc::iovec {
                iov_base: chunk.as_mut_ptr().cast(),
                iov_len: chunk.len(),
            };

            let remote_iov = libc::iovec {
                iov_base: self.remote_ptr,
                iov_len: chunk.len(),
            };

            let read_size = unsafe {
                libc::process_vm_readv(
                    self.pid,
                    &local_iov,
                    1,
                    &remote_iov,
                    1,
                    0,
                )
            };

            let Ok(read_size) = usize::try_from(read_size) else {
                return Err(io::Error::last_os_error());
            };

            // chunk[..read_size] are all initiliazed, but we are only going to advance until '\0'
            let chunk = unsafe { transmute::<&[MaybeUninit<u8>], &[u8]>(&chunk.as_uninit_slice_mut()[..read_size]) }; 
            let Some(nul_index) = chunk.iter().position(|byte| *byte == b'\0') else {
                // No '\0' found, could be a partitial read, advance all of `read_size` and continue reading.
                unsafe { buf.advance_mut(read_size) };
                continue;
            };
            unsafe { buf.advance_mut(nul_index) };
            return Ok(())
        }
    }
    pub fn read_with_buf<const BUF_SIZE: usize, R, F: FnOnce(&[u8]) -> io::Result<R>>(&self, f: F) -> io::Result<R> {
        let mut read_buf: [MaybeUninit<u8>; 32768] = [const { MaybeUninit::uninit() }; 32768];
        let mut read_buf = ReadBuf::uninit(read_buf.as_mut_slice());
        self.read(&mut read_buf)?;
        f(read_buf.filled())
    }
}

impl FromSyscallArg for CStrPtr {
    fn from_syscall_arg(pid: u32, arg: u64) -> io::Result<Self> {
        Ok(Self {
            pid: pid as _,
            remote_ptr: arg as _,
        })
    }
}

#[derive(Debug)]
pub struct Ignored(());
impl FromSyscallArg for Ignored {
    fn from_syscall_arg(_pid: u32, _arg: u64) -> io::Result<Self> {
        Ok(Ignored(()))
    }
}

#[derive(Debug)]
pub struct Fd {
    pid: u32,
    fd: RawFd,
}
impl FromSyscallArg for Fd {
    fn from_syscall_arg(pid: u32, arg: u64) -> io::Result<Self> {
        Ok(Self { pid, fd: arg as _ })
    }
}

impl Fd {
    // TODO: allocate in arena
    pub fn get_path(&self) -> nix::Result<OsString> {
        nix::fcntl::readlink(
            if self.fd == libc::AT_FDCWD {
                format!("/proc/{}/cwd", self.pid)
            } else {
                format!("/proc/{}/fd/{}", self.pid, self.fd)
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
        Ok((T::from_syscall_arg(notif.pid, notif.data.args[0])?,))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg> FromNotify for (T1, T2) {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((
            T1::from_syscall_arg(notif.pid, notif.data.args[0])?,
            T2::from_syscall_arg(notif.pid, notif.data.args[1])?,
        ))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg, T3: FromSyscallArg> FromNotify for (T1, T2, T3) {
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((
            T1::from_syscall_arg(notif.pid, notif.data.args[0])?,
            T2::from_syscall_arg(notif.pid, notif.data.args[1])?,
            T3::from_syscall_arg(notif.pid, notif.data.args[2])?,
        ))
    }
}

impl<T1: FromSyscallArg, T2: FromSyscallArg, T3: FromSyscallArg, T4: FromSyscallArg> FromNotify
    for (T1, T2, T3, T4)
{
    fn from_notify(notif: &seccomp_notif) -> io::Result<Self> {
        Ok((
            T1::from_syscall_arg(notif.pid, notif.data.args[0])?,
            T2::from_syscall_arg(notif.pid, notif.data.args[1])?,
            T3::from_syscall_arg(notif.pid, notif.data.args[2])?,
            T4::from_syscall_arg(notif.pid, notif.data.args[3])?,
        ))
    }
}
