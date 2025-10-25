#[cfg(not(target_os = "linux"))]
fn main() {
    unimplemented!("fspy_test_bin is only for Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    use std::fs::File;

    use nix::fcntl::{AT_FDCWD, OFlag, OpenHow, openat2};
    let args = std::env::args().collect::<Vec<_>>();
    assert!(args.len() == 3, "expected 2 arguments: <action> <file_path>");
    let action = args[1].as_str();
    let path = args[2].as_str();

    match action {
        "open_read" => {
            let _ = File::open(path);
        }
        "open_write" => {
            let _ = File::options().write(true).open(path);
        }
        "open_readwrite" => {
            let _ = File::options().read(true).write(true).open(path);
        }
        "openat2_read" => {
            let _ = openat2(AT_FDCWD, path, OpenHow::new().flags(OFlag::O_RDONLY));
        }
        "openat2_write" => {
            let _ = openat2(AT_FDCWD, path, OpenHow::new().flags(OFlag::O_WRONLY));
        }
        "openat2_readwrite" => {
            let _ = openat2(AT_FDCWD, path, OpenHow::new().flags(OFlag::O_RDWR));
        }
        "readdir" => {
            let mut entries = std::fs::read_dir(path).unwrap();
            let _ = entries.next();
        }
        "stat" => {
            let _ = std::fs::metadata(path);
        }
        "execve" => {
            let _ = std::process::Command::new(path).spawn();
        }
        _ => panic!("unknown action: {}", action),
    }
}
