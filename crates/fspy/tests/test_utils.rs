use std::{
    io,
    path::{Path, PathBuf, StripPrefixError},
};

use fspy::{AccessMode, PathAccessIterable, TrackedChild};

#[track_caller]
pub fn assert_contains(
    accesses: &PathAccessIterable,
    expected_path: &Path,
    expected_mode: AccessMode,
) {
    let found = accesses.iter().any(|access| {
        let Ok(stripped) =
            access.path.strip_path_prefix::<_, Result<PathBuf, StripPrefixError>, _>(
                expected_path,
                |strip_result| strip_result.map(Path::to_path_buf),
            )
        else {
            return false;
        };
        stripped.as_os_str().is_empty() && access.mode == expected_mode
    });
    if !found {
        panic!(
            "Expected to find access to path {:?} with mode {:?}, but it was not found in: {:?}",
            expected_path,
            expected_mode,
            accesses.iter().collect::<Vec<_>>()
        );
    }
}

#[macro_export]
macro_rules! track_child {
    ($body: block) => {{
        const ID: &str =
            ::core::concat!(::core::file!(), ":", ::core::line!(), ":", ::core::column!());
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
        $crate::test_utils::_spawn_with_id(ID)
    }};
}

pub async fn _spawn_with_id(id: &str) -> io::Result<PathAccessIterable> {
    let mut command = fspy::Spy::global()?.new_command(::std::env::current_exe()?);
    command.arg(id);
    let TrackedChild { mut tokio_child, accesses_future } = command.spawn().await?;

    let status = tokio_child.wait().await?;
    let accesses = accesses_future.await?;
    assert!(status.success());
    Ok(accesses)
}
