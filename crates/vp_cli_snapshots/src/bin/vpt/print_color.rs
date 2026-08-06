//! `vpt print-color <color> <text>...` — prints `text` wrapped in an ANSI SGR
//! escape sequence when `FORCE_COLOR` is set to a non-zero value, otherwise
//! prints plain text. Used by e2e tests to verify color-env handling.

pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 2 {
        return Err("Usage: vpt print-color <color> <text>...".into());
    }
    let color = args[0].as_str();
    let text = args[1..].join(" ");

    let force_color = std::env::var("FORCE_COLOR").ok();
    let want_color = match force_color.as_deref() {
        Some("" | "0") | None => false,
        Some(_) => true,
    };

    let code: u8 = match color {
        "red" => 31,
        "green" => 32,
        "yellow" => 33,
        "blue" => 34,
        "magenta" => 35,
        "cyan" => 36,
        other => return Err(format!("Unknown color: {other}").into()),
    };

    if want_color {
        println!("\x1b[{code}m{text}\x1b[0m");
    } else {
        println!("{text}");
    }
    Ok(())
}
