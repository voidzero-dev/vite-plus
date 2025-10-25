#![cfg(target_os = "linux")]

use std::{
    env::{current_dir, set_current_dir},
    error::Error,
    ffi::{CString, OsString},
    io,
    os::unix::ffi::OsStringExt,
    time::Duration,
};

use assertables::assert_contains;
use fspy_seccomp_unotify::{
    impl_handler,
    supervisor::{
        handler::arg::{CStrPtr, Caller, Fd},
        supervise,
    },
    target::install_target,
};
use nix::{
    fcntl::{AT_FDCWD, OFlag, openat},
    sys::stat::Mode,
};
use test_log::test;
use tokio::{process::Command, task::spawn_blocking, time::timeout};
use tracing::{Level, span, trace};

#[derive(Debug, PartialEq, Eq, Clone)]
enum Syscall {
    Openat { at_dir: OsString, path: Option<OsString> },
}

#[derive(Default, Clone, Debug)]
struct SyscallRecorder(Vec<Syscall>);
impl SyscallRecorder {
    fn openat(&mut self, caller: Caller<'_>, (fd, path): (Fd, CStrPtr)) -> io::Result<()> {
        let at_dir = fd.get_path(caller)?;
        let mut buf = [0u8; 40000];
        let path = if let Some(null_pos) = path.read(caller, &mut buf)? {
            Some(OsString::from_vec(buf[..null_pos].to_vec()))
        } else {
            None
        };
        self.0.push(Syscall::Openat { at_dir, path });
        Ok(())
    }
}

impl_handler!(SyscallRecorder: openat,);

async fn run_in_pre_exec(
    mut f: impl FnMut() -> io::Result<()> + Send + Sync + 'static,
) -> Result<Vec<Syscall>, Box<dyn Error>> {
    Ok(timeout(Duration::from_secs(5), async move {
        let mut cmd = Command::new("/bin/echo");
        let supervisor = supervise::<SyscallRecorder>()?;

        let payload = supervisor.payload().clone();

        unsafe {
            cmd.pre_exec(move || {
                install_target(&payload)?;
                f()?;
                Ok(())
            });
        }
        let child_fut = spawn_blocking(move || {
            let _span = span!(Level::TRACE, "spawn test child process");
            cmd.spawn()
        });

        let exit_status = child_fut.await.unwrap()?.wait().await?;
        trace!("test child process exited with status: {:?}", exit_status);

        trace!("waiting for handler to finish and test child process to exit");

        assert!(exit_status.success());

        let recorders = supervisor.stop().await?;
        trace!("{} recorders awaited", recorders.len());

        let syscalls = recorders.into_iter().map(|recorder| recorder.0.into_iter()).flatten();
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
    assert_contains!(syscalls, &Syscall::Openat { at_dir: "/".into(), path: Some("/home".into()) });
    assert_contains!(
        syscalls,
        &Syscall::Openat { at_dir: "/home".into(), path: Some("open_at_home".into()) }
    );
    assert_contains!(
        syscalls,
        &Syscall::Openat { at_dir: "/".into(), path: Some("openat_cwd".into()) }
    );
    Ok(())
}

#[tokio::test]
async fn path_long() -> Result<(), Box<dyn Error>> {
    let long_path = [b'a'].repeat(30000);
    let long_path_cstr = CString::new(long_path.as_slice()).unwrap();
    let syscalls = run_in_pre_exec(move || {
        let _ = openat(AT_FDCWD, long_path_cstr.as_c_str(), OFlag::O_RDONLY, Mode::empty());
        Ok(())
    })
    .await?;
    assert_contains!(
        syscalls,
        &Syscall::Openat {
            at_dir: current_dir().unwrap().into(),
            path: Some(OsString::from_vec(long_path)),
        }
    );
    Ok(())
}

#[tokio::test]
async fn path_overflow() -> Result<(), Box<dyn Error>> {
    let long_path = [b'a'].repeat(40000);
    let long_path_cstr = CString::new(long_path.as_slice()).unwrap();
    let syscalls = run_in_pre_exec(move || {
        let _ = openat(AT_FDCWD, long_path_cstr.as_c_str(), OFlag::O_RDONLY, Mode::empty());
        Ok(())
    })
    .await?;
    assert_contains!(
        syscalls,
        &Syscall::Openat { at_dir: current_dir().unwrap().into(), path: None }
    );
    Ok(())
}
