mod test_utils;

use std::{
    env::{current_dir, vars_os},
    io,
    path::Path,
};
use test_utils::assert_contains;

use fspy::{AccessMode, PathAccessIterable, TrackedChild};

async fn track_node_script(script: &str) -> io::Result<PathAccessIterable> {
    let mut command = fspy::Spy::global()?.new_command("node");
    command
        .arg("-e")
        .envs(vars_os()) // https://github.com/jdx/mise/discussions/5968
        .arg(script);
    let TrackedChild {
        mut tokio_child,
        accesses_future,
    } = command.spawn().await?;

    let acceses = accesses_future.await?;
    let status = tokio_child.wait().await?;
    assert!(status.success());
    Ok(acceses)
}

#[tokio::test]
async fn read_sync() -> io::Result<()> {
    let accesses = track_node_script("try { fs.readFileSync('hello') } catch {}").await?;
    assert_contains(
        &accesses,
        current_dir().unwrap().join("hello").as_path(),
        AccessMode::Read,
    );
    Ok(())
}

#[tokio::test]
async fn read_dir_sync() -> io::Result<()> {
    let accesses = track_node_script("try { fs.readdirSync('.') } catch {}").await?;
    assert_contains(&accesses, &current_dir().unwrap(), AccessMode::ReadDir);
    Ok(())
}

#[tokio::test]
async fn subprocess() -> io::Result<()> {
    let cmd = if cfg!(windows) {
        r#"'cmd', ['/c', 'type hello']"#
    } else {
        r#"'/bin/sh', ['-c', 'cat hello']"#
    };
    let accesses = track_node_script(&format!(
        "try {{ child_process.spawnSync({cmd}, {{ stdio: 'ignore' }}) }} catch {{}}"
    ))
    .await?;
    assert_contains(
        &accesses,
        current_dir().unwrap().join("hello").as_path(),
        AccessMode::Read,
    );
    Ok(())
}
