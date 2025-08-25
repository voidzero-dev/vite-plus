use std::{env::args_os, ffi::OsStr, io, path::PathBuf, pin::Pin};

use fspy::{AccessMode, TrackedChild};
use tokio::{
    fs::File,
    io::{AsyncWrite, stdout},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = args_os();
    let _ = args.next();
    assert_eq!(args.next().as_deref(), Some(OsStr::new("-o")));

    let out_path = args.next().unwrap();

    let program = PathBuf::from(args.next().unwrap());

    let spy = fspy::Spy::global()?;

    let mut command = spy.new_command(program);
    command.envs(std::env::vars_os()).args(args);

    let TrackedChild {
        mut tokio_child,
        accesses_future,
    } = command.spawn().await?;

    let acceses = accesses_future.await?;

    let mut path_count = 0usize;
    let out_file: Pin<Box<dyn AsyncWrite>> = if out_path == "-" {
        Box::pin(stdout())
    } else {
        Box::pin(File::create(out_path).await?)
    };

    let mut csv_writer = csv_async::AsyncWriter::from_writer(out_file);

    for acc in acceses.iter() {
        path_count += 1;
        csv_writer
            .write_record(&[
                acc.path
                    .to_cow_os_str()
                    .to_string_lossy()
                    .as_ref()
                    .as_bytes(),
                match acc.mode {
                    AccessMode::Read => b"read".as_slice(),
                    AccessMode::ReadWrite => b"readwrite",
                    AccessMode::Write => b"write",
                    AccessMode::ReadDir => b"readdir",
                },
            ])
            .await?;
    }
    csv_writer.flush().await?;

    let output = tokio_child.wait().await?;
    eprintln!("\nfspy: {} paths accessed. {}", path_count, output);
    Ok(())
}
