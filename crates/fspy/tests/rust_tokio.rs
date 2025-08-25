mod test_utils;

use std::{env::current_dir, io, path::Path, process::Stdio};

use fspy::AccessMode;
use test_utils::{assert_contains, track_child};
use tokio::fs::OpenOptions;

#[tokio::test]
async fn open_read() -> io::Result<()> {
    let accesses = track_child!({
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(async {
                tokio::fs::File::open("hello").await;
            });
    })
    .await?;
    assert_contains(
        &accesses,
        current_dir().unwrap().join("hello").as_path(),
        AccessMode::Read,
    );

    Ok(())
}

#[tokio::test]
async fn open_write() -> io::Result<()> {
    let accesses = track_child!({
        let path = format!("{}/hello", env!("CARGO_TARGET_TMPDIR"));

        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(async {
                OpenOptions::new().write(true).open(path).await;
            });
    })
    .await?;
    assert_contains(
        &accesses,
        Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("hello")
            .as_path(),
        AccessMode::Write,
    );

    Ok(())
}

#[tokio::test]
async fn readdir() -> io::Result<()> {
    let accesses = track_child!({
        let path = format!("{}/hello", env!("CARGO_TARGET_TMPDIR"));

        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(async {
                tokio::fs::read_dir(path).await;
            });
    })
    .await?;
    assert_contains(
        &accesses,
        Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("hello")
            .as_path(),
        AccessMode::ReadDir,
    );

    Ok(())
}

#[tokio::test]
async fn subprocess() -> io::Result<()> {
    let accesses = track_child!({
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(async {
                let mut command = if cfg!(windows) {
                    let mut command = tokio::process::Command::new("cmd");
                    command.arg("/c").arg("type hello");
                    command
                } else {
                    let mut command = tokio::process::Command::new("/bin/sh");
                    command.arg("-c").arg("cat hello");
                    command
                };
                command
                    .stdout(Stdio::null())
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap()
                    .wait()
                    .await
                    .unwrap();
            });
    })
    .await?;
    assert_contains(
        &accesses,
        current_dir().unwrap().join("hello").as_path(),
        AccessMode::Read,
    );

    Ok(())
}
