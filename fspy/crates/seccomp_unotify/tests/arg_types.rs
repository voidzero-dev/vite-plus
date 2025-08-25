#![cfg(target_os = "linux")]

use assertables::assert_contains;
use nix::fcntl::{AT_FDCWD, OFlag, openat};
use nix::sys::stat::Mode;
use seccomp_unotify::supervisor::Supervisor;
use tokio::time::timeout;

use std::env::{current_dir, set_current_dir};
use std::error::Error;
use std::ffi::OsString;
use std::ffi::{CString, OsStr};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::time::Duration;
use test_log::test;
use tracing::{Level, span, trace};

use seccomp_unotify::{
    impl_handler,
    supervisor::{
        handler::arg::{CStrPtr, Fd},
        supervise,
    },
    target::install_target,
};
use tokio::{process::Command, task::spawn_blocking};

#[derive(Debug, PartialEq, Eq, Clone)]
enum Syscall {
    Openat { at_dir: OsString, path: OsString },
}

#[derive(Default, Clone, Debug)]
struct SyscallRecorder(Vec<Syscall>);
impl SyscallRecorder {
    fn openat(&mut self, (fd, path): (Fd, CStrPtr)) -> io::Result<()> {
        let at_dir = fd.get_path()?;
        let path = path.read_with_buf::<32768, _, _>(|path: &[u8]| {
            Ok(OsStr::from_bytes(path).to_os_string())
        })?;
        self.0.push(Syscall::Openat { at_dir, path });
        Ok(())
    }
}

impl_handler!(SyscallRecorder, openat);

async fn run_in_pre_exec(
    mut f: impl FnMut() -> io::Result<()> + Send + Sync + 'static,
) -> Result<Vec<Syscall>, Box<dyn Error>> {
    Ok(timeout(Duration::from_secs(5), async move {
        let mut cmd = Command::new("/bin/echo");
        let Supervisor {
            payload,
            handling_loop,
            mut pre_exec,
        } = supervise::<SyscallRecorder>()?;

        unsafe {
            cmd.pre_exec(move || {
                install_target(&payload)?;
                pre_exec.run()?;
                f()?;
                Ok(())
            });
        }
        let child_fut = spawn_blocking(move || {
            let _span = span!(Level::TRACE, "spawn test child process");
            cmd.spawn()
        });
        trace!("waiting for handler to finish and test child process to exit");
        let (recorders, exit_status) = futures_util::future::try_join(
            async move {
                let recorders = handling_loop.await?;
                trace!("{} recorders awaited", recorders.len());
                Ok(recorders)
            },
            async move {
                let exit_status = child_fut.await.unwrap()?.wait().await?;
                trace!("test child process exited with status: {:?}", exit_status);
                io::Result::Ok(exit_status)
            },
        )
        .await?;

        assert!(exit_status.success());

        let syscalls = recorders
            .into_iter()
            .map(|recorder| recorder.0.into_iter())
            .flatten();
        io::Result::Ok(syscalls.collect())
    })
    .await??)
}

#[test(tokio::test)]
async fn fd_and_path() -> Result<(), Box<dyn Error>> {
    let syscalls = run_in_pre_exec(|| {
        set_current_dir("/")?;
        let home_fd = nix::fcntl::open(c"/home", OFlag::O_PATH, Mode::empty())?;
        let _ = openat(home_fd, c"open_at_home", OFlag::O_RDONLY, Mode::empty());
        let _ = openat(AT_FDCWD, c"openat_cwd", OFlag::O_RDONLY, Mode::empty());
        Ok(())
    })
    .await?;
    assert_contains!(
        syscalls,
        &Syscall::Openat {
            at_dir: "/".into(),
            path: "/home".into(),
        }
    );
    assert_contains!(
        syscalls,
        &Syscall::Openat {
            at_dir: "/home".into(),
            path: "open_at_home".into(),
        }
    );
    assert_contains!(
        syscalls,
        &Syscall::Openat {
            at_dir: "/".into(),
            path: "openat_cwd".into(),
        }
    );
    Ok(())
}

#[tokio::test]
async fn path_long() -> Result<(), Box<dyn Error>> {
    let long_path = [b'a'].repeat(30000);
    let long_path_cstr = CString::new(long_path.as_slice()).unwrap();
    let syscalls = run_in_pre_exec(move || {
        let _ = openat(
            AT_FDCWD,
            long_path_cstr.as_c_str(),
            OFlag::O_RDONLY,
            Mode::empty(),
        );
        Ok(())
    })
    .await?;
    assert_contains!(
        syscalls,
        &Syscall::Openat {
            at_dir: current_dir().unwrap().into(),
            path: OsString::from_vec(long_path),
        }
    );
    Ok(())
}

#[tokio::test]
async fn path_overflow() -> Result<(), Box<dyn Error>> {
    let long_path = [b'a'].repeat(40000);
    let long_path_cstr = CString::new(long_path.as_slice()).unwrap();
    let ret = run_in_pre_exec(move || {
        let _ = openat(
            AT_FDCWD,
            long_path_cstr.as_c_str(),
            OFlag::O_RDONLY,
            Mode::empty(),
        );
        Ok(())
    })
    .await;
    let err = ret.unwrap_err();
    assert_eq!(
        err.downcast::<io::Error>().unwrap().kind(),
        io::ErrorKind::InvalidFilename
    );
    Ok(())
}
