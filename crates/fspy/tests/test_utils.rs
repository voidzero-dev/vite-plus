use std::{ffi::OsStr, io, path::Path};

use fspy::{AccessMode, PathAccessIterable, TrackedChild};

#[track_caller]
pub fn assert_contains(
    accesses: &PathAccessIterable,
    expected_path: &Path,
    expected_mode: AccessMode,
) {
    accesses
        .iter()
        .find(|access| {
            let path = access.path.to_cow_os_str();
            let mut path: &OsStr = path.as_ref();
            if cfg!(windows) {
                let mut path_bytes = path.as_encoded_bytes();
                for prefix in [br#"\\.\"#, br#"\\?\"#, br#"\??\"#] {
                    if let Some(stripped_path_bytes) = path_bytes.strip_prefix(prefix) {
                        path_bytes = stripped_path_bytes;
                        break;
                    }
                }
                path = unsafe { OsStr::from_encoded_bytes_unchecked(path_bytes) };
            }
            Path::new(path) == expected_path && access.mode == expected_mode
        })
        .unwrap();
}

macro_rules! track_child {
    ($body: block) => {{
        const ID: &str = ::core::concat!(
            ::core::file!(),
            ":",
            ::core::line!(),
            ":",
            ::core::column!()
        );
        #[ctor::ctor]
        unsafe fn init() {
            let mut args = ::std::env::args();
            let Some(_) = args.next() else {
                return;
            };
            let Some(current_id) = args.next() else {
                return;
            };
            if current_id == ID {
                $body;
                ::std::process::exit(0);
            }
        }
        $crate::test_utils::spawn_with_id(ID)
    }};
}

pub async fn spawn_with_id(id: &str) -> io::Result<PathAccessIterable> {
    let mut command = fspy::Spy::global()?.new_command(::std::env::current_exe()?);
    command.arg(id);
    let TrackedChild {
        mut tokio_child,
        accesses_future,
    } = command.spawn().await?;

    let acceses = accesses_future.await?;
    let status = tokio_child.wait().await?;
    assert!(status.success());
    Ok(acceses)
}

pub(crate) use track_child;


