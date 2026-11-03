use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

/// head-lines `<count>` -- `<command>` \[`<args>`...\]
///
/// Reads and prints the first `<count>` lines from the child's stdout, closes
/// the pipe, then exits with the child's exit code.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sep = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("Usage: vpt head-lines <count> -- <command> [args...]")?;
    if sep != 1 || args.len() <= sep + 1 {
        return Err("Usage: vpt head-lines <count> -- <command> [args...]".into());
    }

    let count = args[0].parse::<usize>()?;
    if count == 0 {
        return Err("head-lines count must be greater than zero".into());
    }

    let cmd_args = &args[sep + 1..];
    let mut child =
        Command::new(&cmd_args[0]).args(&cmd_args[1..]).stdout(Stdio::piped()).spawn()?;
    let mut child_stdout = child.stdout.take().unwrap();
    let mut captured = Vec::new();
    let mut lines = 0;

    while lines < count {
        let mut byte = [0];
        if child_stdout.read(&mut byte)? == 0 {
            break;
        }
        captured.push(byte[0]);
        if byte[0] == b'\n' {
            lines += 1;
        }
    }

    // Closing the read end reproduces a consumer such as `head` exiting early.
    drop(child_stdout);
    let status = child.wait()?;

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&captured)?;
    stdout.flush()?;

    std::process::exit(crate::exit_code::exit_code_from_status(status));
}
