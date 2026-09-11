/// stat-file `<path>...` `[--assert <state>|--assert-not <state>]`
///
/// Reports each entry's type, not just existence, so migrated `test -f` /
/// `test -d` / `test -L` assertions keep their predicate fidelity in
/// snapshots. A symlink reports `symlink` without following it (shell
/// `test -L` semantics), regardless of whether the target exists. The
/// assert flags (states: `file`, `dir`, `symlink`, `missing`) additionally
/// fail the step on mismatch, preserving shell `test` guard semantics under
/// the runner's line-boundary failure flow.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut paths: Vec<&str> = Vec::new();
    let mut expect: Option<(&str, bool)> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            flag @ ("--assert" | "--assert-not") => {
                i += 1;
                let state = args.get(i).ok_or("--assert requires a state")?.as_str();
                if !matches!(state, "file" | "dir" | "symlink" | "missing") {
                    return Err(format!(
                        "unknown state `{state}`; expected file, dir, symlink, or missing"
                    )
                    .into());
                }
                expect = Some((state, flag == "--assert-not"));
            }
            path => paths.push(path),
        }
        i += 1;
    }
    if paths.is_empty() {
        return Err("Usage: vpt stat-file <path>... [--assert <state>|--assert-not <state>]".into());
    }

    let mut failed = false;
    for file in paths {
        let state = match std::fs::symlink_metadata(file) {
            Ok(meta) if meta.is_symlink() => "symlink",
            Ok(meta) if meta.is_dir() => "dir",
            Ok(_) => "file",
            Err(_) => "missing",
        };
        println!("{file}: {state}");
        if let Some((want, negated)) = expect
            && (state == want) == negated
        {
            failed = true;
        }
    }
    if failed {
        return Err("stat-file assertion failed".into());
    }
    Ok(())
}
