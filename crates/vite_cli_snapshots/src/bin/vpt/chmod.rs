/// chmod `<octal-mode>|+x` `<path>`
///
/// Sets POSIX permission bits; `+x` adds the execute bits to the current
/// mode (for example, `chmod +x hook.mjs`). Windows
/// treats it as a validated no-op: the mode and target are still checked,
/// so a typo or a failed earlier setup step fails on every platform.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 2 {
        return Err("Usage: vpt chmod <octal-mode>|+x <path>".into());
    }
    if args[0] != "+x" {
        u32::from_str_radix(&args[0], 8)?;
    }
    let metadata = std::fs::metadata(&args[1])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if args[0] == "+x" {
            metadata.permissions().mode() | 0o111
        } else {
            u32::from_str_radix(&args[0], 8)?
        };
        std::fs::set_permissions(&args[1], std::fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    drop(metadata);
    Ok(())
}
