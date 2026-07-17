use std::{
    fs::File,
    io::{self, Read},
    os::fd::AsFd,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    poll::{PollFd, PollFlags, PollTimeout, poll},
    sys::socket::{
        AddressFamily, SockFlag, SockType, setsockopt, socketpair,
        sockopt::{RcvBuf, SndBuf},
    },
};

const REQUESTED_SOCKET_BUFFER: usize = 1024;
const DRAIN_CHUNK: usize = 1024;

#[derive(Debug, Default)]
struct Options {
    digest: Option<(usize, usize)>,
    command: Vec<String>,
}

/// Run a command with a deliberately non-blocking, backpressured stdout.
///
/// The reader consumes one small chunk whenever the channel fills. That lets a
/// blocking writer advance while keeping enough pressure for a non-blocking
/// writer to encounter `EAGAIN`.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options(args)?;
    let (reader_fd, writer_fd) =
        socketpair(AddressFamily::Unix, SockType::Stream, None, SockFlag::empty())?;
    setsockopt(&reader_fd, RcvBuf, &REQUESTED_SOCKET_BUFFER)?;
    setsockopt(&writer_fd, SndBuf, &REQUESTED_SOCKET_BUFFER)?;

    let mut reader = File::from(reader_fd);
    let writer = File::from(writer_fd);
    set_nonblocking(&writer, true)?;

    let retained_writer = writer.try_clone()?;

    let mut child = Command::new(&options.command[0])
        .args(&options.command[1..])
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().ok_or("failed to capture child stderr")?;
    let stderr_thread = thread::spawn(move || -> io::Result<Vec<u8>> {
        let mut stderr = stderr;
        let mut captured = Vec::new();
        stderr.read_to_end(&mut captured)?;
        Ok(captured)
    });

    let (status, stdout, saw_nonblocking_backpressure) =
        capture_backpressured_stdout(&mut child, &mut reader, retained_writer)?;
    let stderr = stderr_thread.join().map_err(|_| "stderr reader thread panicked")??;

    if saw_nonblocking_backpressure {
        eprintln!("backpressure-run detected truncated child output under stdio backpressure");
        std::process::exit(1);
    }

    replay("stdout", &stdout, options.digest);
    replay("stderr", &stderr, options.digest);

    std::process::exit(status.code().unwrap_or(1));
}

fn parse_options(args: &[String]) -> Result<Options, Box<dyn std::error::Error>> {
    let Some(separator) = args.iter().position(|arg| arg == "--") else {
        return Err(usage().into());
    };

    let mut options = Options { command: args[separator + 1..].to_vec(), ..Options::default() };
    if options.command.is_empty() {
        return Err(usage().into());
    }

    let mut index = 0;
    while index < separator {
        match args[index].as_str() {
            "--digest" => {
                let value = args.get(index + 1).ok_or_else(usage)?;
                options.digest = Some(parse_digest(value)?);
                index += 2;
            }
            _ => return Err(usage().into()),
        }
    }

    Ok(options)
}

fn usage() -> String {
    "Usage: vpt backpressure-run [--digest <head>,<tail>] -- <command> [args...]".to_owned()
}

fn parse_digest(value: &str) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let Some((head, tail)) = value.split_once(',') else {
        return Err("--digest must be <head>,<tail>".into());
    };
    Ok((head.parse()?, tail.parse()?))
}

fn set_nonblocking(fd: &impl AsFd, nonblocking: bool) -> io::Result<()> {
    let flags = fcntl(fd, FcntlArg::F_GETFL).map_err(io::Error::from)?;
    let mut flags = OFlag::from_bits_retain(flags);
    flags.set(OFlag::O_NONBLOCK, nonblocking);
    fcntl(fd, FcntlArg::F_SETFL(flags)).map_err(io::Error::from)?;
    Ok(())
}

fn capture_backpressured_stdout(
    child: &mut std::process::Child,
    reader: &mut File,
    retained_writer: File,
) -> io::Result<(ExitStatus, Vec<u8>, bool)> {
    let mut retained_writer = Some(retained_writer);
    let mut captured = Vec::new();
    let mut saw_nonblocking_backpressure = false;

    loop {
        if let Some(status) = child.try_wait()? {
            retained_writer.take();
            reader.read_to_end(&mut captured)?;
            return Ok((status, captured, saw_nonblocking_backpressure));
        }

        let writer = retained_writer.as_ref().expect("writer is retained until child exit");
        if !is_writable(writer)? {
            saw_nonblocking_backpressure |= is_nonblocking(writer)?;

            let mut chunk = [0_u8; DRAIN_CHUNK];
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "stdout socket closed before the child exited",
                ));
            }
            captured.extend_from_slice(&chunk[..read]);
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn is_writable(fd: &impl AsFd) -> io::Result<bool> {
    let mut fds = [PollFd::new(fd.as_fd(), PollFlags::POLLOUT)];
    loop {
        match poll(&mut fds, PollTimeout::ZERO) {
            Ok(ready) => return Ok(ready > 0),
            Err(nix::errno::Errno::EINTR) => {}
            Err(error) => return Err(io::Error::from(error)),
        }
    }
}

fn is_nonblocking(fd: &impl AsFd) -> io::Result<bool> {
    let flags = fcntl(fd, FcntlArg::F_GETFL).map_err(io::Error::from)?;
    Ok(OFlag::from_bits_retain(flags).contains(OFlag::O_NONBLOCK))
}

fn replay(name: &str, bytes: &[u8], digest: Option<(usize, usize)>) {
    println!("--- {name} ---");
    let text = String::from_utf8_lossy(bytes);
    if let Some((head, tail)) = digest {
        let lines: Vec<_> = text.lines().collect();
        println!("{name}: {} lines", lines.len());
        if lines.len() <= head + tail {
            for line in lines {
                println!("{line}");
            }
        } else {
            for line in &lines[..head] {
                println!("{line}");
            }
            println!("... {} lines elided ...", lines.len() - head - tail);
            for line in &lines[lines.len() - tail..] {
                println!("{line}");
            }
        }
    } else {
        print!("{text}");
        if !text.is_empty() && !text.ends_with('\n') {
            println!();
        }
    }
}
