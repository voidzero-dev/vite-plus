// This is a standalone test utility binary that deliberately uses std types
// rather than the project's custom types (vite_str, vite_path, etc.). Its
// subcommand names and semantics follow vite-task's `vtt` multitool verbatim
// wherever they overlap, so fixtures and habits transfer between the repos;
// `chmod`, `json-edit`, and `probe` are vite-plus additions.
#![expect(clippy::disallowed_types, reason = "standalone test utility uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test utility uses std macros")]
#![expect(clippy::disallowed_methods, reason = "standalone test utility uses std methods")]
#![expect(clippy::print_stderr, reason = "CLI tool error output")]
#![expect(clippy::print_stdout, reason = "CLI tool output")]

mod barrier;
mod check_tty;
mod chmod;
mod cp;
mod exit;
mod exit_on_ctrlc;
mod grep_file;
mod json_edit;
mod list_dir;
mod mkdir;
mod pipe_stdin;
mod print;
mod print_color;
mod print_cwd;
mod print_env;
mod print_file;
mod print_native_path;
mod probe;
mod read_stdin;
mod replace_file_content;
mod report_orphan_on_ctrlc;
mod rm;
mod stat_file;
mod touch_file;
mod wait_file;
mod write_file;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vpt <subcommand> [args...]");
        eprintln!(
            "Subcommands: barrier, check-tty, chmod, cp, exit, exit-on-ctrlc, grep-file, json-edit, list-dir, mkdir, pipe-stdin, print, print-color, print-cwd, print-env, print-file, print-native-path, probe, read-stdin, replace-file-content, report-orphan-on-ctrlc, rm, stat-file, touch-file, wait-file, write-file"
        );
        std::process::exit(1);
    }

    let result: Result<(), Box<dyn std::error::Error>> = match args[1].as_str() {
        "barrier" => barrier::run(&args[2..]),
        "check-tty" => {
            check_tty::run();
            Ok(())
        }
        "chmod" => chmod::run(&args[2..]),
        "cp" => cp::run(&args[2..]),
        "exit" => exit::run(&args[2..]),
        "exit-on-ctrlc" => exit_on_ctrlc::run(),
        "grep-file" => grep_file::run(&args[2..]),
        "json-edit" => json_edit::run(&args[2..]),
        "list-dir" => list_dir::run(&args[2..]),
        "mkdir" => mkdir::run(&args[2..]),
        "pipe-stdin" => pipe_stdin::run(&args[2..]),
        "print" => {
            print::run(&args[2..]);
            Ok(())
        }
        "print-color" => print_color::run(&args[2..]),
        "print-cwd" => print_cwd::run(),
        "print-env" => print_env::run(&args[2..]),
        "print-file" => print_file::run(&args[2..]),
        "print-native-path" => {
            print_native_path::run(&args[2..]);
            Ok(())
        }
        "probe" => probe::run(),
        "read-stdin" => read_stdin::run(),
        "replace-file-content" => replace_file_content::run(&args[2..]),
        "report-orphan-on-ctrlc" => report_orphan_on_ctrlc::run(&args[2..]),
        "rm" => rm::run(&args[2..]),
        "stat-file" => stat_file::run(&args[2..]),
        "touch-file" => touch_file::run(&args[2..]),
        "wait-file" => wait_file::run(&args[2..]),
        "write-file" => write_file::run(&args[2..]),
        other => {
            eprintln!("Unknown subcommand: {other}");
            std::process::exit(1);
        }
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
