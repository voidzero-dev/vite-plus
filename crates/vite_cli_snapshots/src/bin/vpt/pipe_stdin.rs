/// pipe-stdin `<data>` -- `<command>` \[`<args>`...\]
///
/// Spawns `<command>` with `<data>` piped to its stdin, then exits with
/// the child's exit code. If `<data>` is empty, an empty stdin is provided.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sep = args
        .iter()
        .position(|a| a == "--")
        .ok_or("Usage: vpt pipe-stdin <data> -- <command> [args...]")?;
    let data = &args[..sep].join(" ");
    let cmd_args = &args[sep + 1..];
    if cmd_args.is_empty() {
        return Err("Usage: vpt pipe-stdin <data> -- <command> [args...]".into());
    }

    let mut child = std::process::Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        // Empty data means genuinely empty stdin (immediate EOF), as
        // documented; the newline only terminates actual input.
        if !data.is_empty() {
            stdin.write_all(data.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
        // stdin is closed when dropped, signaling EOF
    }

    let status = child.wait()?;
    std::process::exit(status.code().unwrap_or(1));
}
