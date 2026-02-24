/// Parsed exec flags.
pub(super) struct ExecFlags {
    pub shell_mode: bool,
    pub help: bool,
    pub recursive: bool,
    pub filters: Vec<String>,
    pub parallel: bool,
    pub reverse: bool,
    pub resume_from: Option<String>,
    pub report_summary: bool,
    pub include_workspace_root: bool,
    pub workspace_root: bool,
}

/// Parse exec-specific flags from argument slice.
///
/// Handles: -c/--shell-mode, -h/--help, -r/--recursive, --filter, --parallel,
/// and leading -- stripping.
/// All other arguments (including unknown flags) are treated as positional.
pub(super) fn parse_exec_args(args: &[String]) -> (ExecFlags, Vec<String>) {
    let mut flags = ExecFlags {
        shell_mode: false,
        help: false,
        recursive: false,
        filters: Vec::new(),
        parallel: false,
        reverse: false,
        resume_from: None,
        report_summary: false,
        include_workspace_root: false,
        workspace_root: false,
    };
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Strip leading --
        if arg == "--" {
            positional.extend_from_slice(&args[i + 1..]);
            break;
        }

        // Once we see a non-flag argument, everything else is positional
        if !arg.starts_with('-') {
            positional.extend_from_slice(&args[i..]);
            break;
        }

        match arg.as_str() {
            "-c" | "--shell-mode" => {
                flags.shell_mode = true;
            }
            "-h" | "--help" => {
                flags.help = true;
            }
            "-r" | "--recursive" => {
                flags.recursive = true;
            }
            "--parallel" => {
                flags.parallel = true;
            }
            "--reverse" => {
                flags.reverse = true;
            }
            "--resume-from" => {
                i += 1;
                if i < args.len() {
                    flags.resume_from = Some(args[i].clone());
                }
            }
            "--report-summary" => {
                flags.report_summary = true;
            }
            "--include-workspace-root" => {
                flags.include_workspace_root = true;
            }
            "-w" | "--workspace-root" => {
                flags.workspace_root = true;
            }
            "--filter" => {
                i += 1;
                if i < args.len() {
                    flags.filters.push(args[i].clone());
                }
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--filter=") {
                    flags.filters.push(value.to_string());
                } else if let Some(value) = arg.strip_prefix("--resume-from=") {
                    flags.resume_from = Some(value.to_string());
                } else {
                    // Unknown flag — treat as start of positional args
                    positional.extend_from_slice(&args[i..]);
                    break;
                }
            }
        }

        i += 1;
    }

    (flags, positional)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_args_recursive() {
        let args: Vec<String> =
            vec!["-r", "--", "echo", "hello"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(!flags.shell_mode);
        assert!(!flags.parallel);
        assert!(flags.filters.is_empty());
        assert_eq!(positional, vec!["echo", "hello"]);
    }

    #[test]
    fn test_parse_exec_args_filter() {
        let args: Vec<String> =
            vec!["--filter", "app-*", "--", "echo", "hi"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(!flags.recursive);
        assert_eq!(flags.filters, vec!["app-*"]);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_multiple_filters() {
        let args: Vec<String> = vec!["--filter", "app-*", "--filter", "lib-*", "--", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (flags, positional) = parse_exec_args(&args);
        assert_eq!(flags.filters, vec!["app-*", "lib-*"]);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_parallel() {
        let args: Vec<String> =
            vec!["-r", "--parallel", "--", "echo", "test"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(flags.parallel);
        assert_eq!(positional, vec!["echo", "test"]);
    }

    #[test]
    fn test_parse_exec_args_combined_flags() {
        let args: Vec<String> =
            vec!["-r", "-c", "--parallel", "echo hello"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(flags.shell_mode);
        assert!(flags.parallel);
        assert_eq!(positional, vec!["echo hello"]);
    }

    #[test]
    fn test_parse_exec_args_filter_with_recursive() {
        let args: Vec<String> =
            vec!["-r", "--filter", "app-a", "--", "tsc"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert_eq!(flags.filters, vec!["app-a"]);
        assert_eq!(positional, vec!["tsc"]);
    }

    #[test]
    fn test_parse_exec_args_reverse() {
        let args: Vec<String> =
            vec!["-r", "--reverse", "--", "echo", "hi"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(flags.reverse);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_resume_from() {
        let args: Vec<String> = vec!["-r", "--resume-from", "lib-c", "--", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert_eq!(flags.resume_from.as_deref(), Some("lib-c"));
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_report_summary() {
        let args: Vec<String> = vec!["-r", "--report-summary", "--", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(flags.report_summary);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_filter_equals() {
        let args: Vec<String> =
            vec!["--filter=app-*", "--", "echo", "hi"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert_eq!(flags.filters, vec!["app-*"]);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_resume_from_equals() {
        let args: Vec<String> = vec!["-r", "--resume-from=lib-c", "--", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert_eq!(flags.resume_from.as_deref(), Some("lib-c"));
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_include_workspace_root() {
        let args: Vec<String> = vec!["-r", "--include-workspace-root", "--", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.recursive);
        assert!(flags.include_workspace_root);
        assert!(!flags.workspace_root);
        assert_eq!(positional, vec!["echo", "hi"]);
    }

    #[test]
    fn test_parse_exec_args_workspace_root() {
        let args: Vec<String> =
            vec!["-w", "--", "echo", "hi"].iter().map(|s| s.to_string()).collect();
        let (flags, positional) = parse_exec_args(&args);
        assert!(flags.workspace_root);
        assert!(!flags.recursive);
        assert!(!flags.include_workspace_root);
        assert_eq!(positional, vec!["echo", "hi"]);
    }
}
