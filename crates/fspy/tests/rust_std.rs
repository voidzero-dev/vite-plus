mod test_utils;

use fspy::{AccessMode, Command};
use std::{
    env::current_dir,
    fs::{File, OpenOptions},
    io::{self, Stdin},
    path::Path,
    process::Stdio,
};
use test_utils::{assert_contains, track_child};

#[tokio::test]
async fn open_read() -> io::Result<()> {
    let accesses = track_child!({
        File::open("hello");
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
        OpenOptions::new().write(true).open(path);
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
        std::fs::read_dir(path);
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
        let mut command = if cfg!(windows) {
            let mut command = std::process::Command::new("cmd");
            command.arg("/c").arg("type hello");
            command
        } else {
            let mut command = std::process::Command::new("/bin/sh");
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
            .unwrap();
    })
    .await?;
    assert_contains(
        &accesses,
        current_dir().unwrap().join("hello").as_path(),
        AccessMode::Read,
    );

    Ok(())
}
