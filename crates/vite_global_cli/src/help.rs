//! Unified help rendering for the global CLI.

use std::{fmt::Write as _, io::IsTerminal};

use clap::{CommandFactory, error::ErrorKind};
use owo_colors::OwoColorize;

#[derive(Clone, Debug)]
pub struct HelpDoc {
    pub usage: &'static str,
    pub summary: Vec<&'static str>,
    pub sections: Vec<HelpSection>,
    pub documentation_url: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub enum HelpSection {
    Rows { title: &'static str, rows: Vec<HelpRow> },
    Lines { title: &'static str, lines: Vec<&'static str> },
}

#[derive(Clone, Debug)]
pub struct HelpRow {
    pub label: &'static str,
    pub description: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct OwnedHelpDoc {
    usage: String,
    summary: Vec<String>,
    sections: Vec<OwnedHelpSection>,
    documentation_url: Option<String>,
}

#[derive(Clone, Debug)]
enum OwnedHelpSection {
    Rows { title: String, rows: Vec<OwnedHelpRow> },
    Lines { title: String, lines: Vec<String> },
}

#[derive(Clone, Debug)]
struct OwnedHelpRow {
    label: String,
    description: Vec<String>,
}

fn row(label: &'static str, description: &'static str) -> HelpRow {
    HelpRow { label, description: vec![description] }
}

fn section_rows(title: &'static str, rows: Vec<HelpRow>) -> HelpSection {
    HelpSection::Rows { title, rows }
}

fn section_lines(title: &'static str, lines: Vec<&'static str>) -> HelpSection {
    HelpSection::Lines { title, lines }
}

fn documentation_url_for_command_path(command_path: &[&str]) -> Option<&'static str> {
    match command_path {
        [] => Some("https://viteplus.dev/guide/"),
        ["create"] => Some("https://viteplus.dev/guide/create"),
        ["migrate"] => Some("https://viteplus.dev/guide/migrate"),
        ["config"] | ["staged"] => Some("https://viteplus.dev/guide/commit-hooks"),
        [
            "install" | "add" | "remove" | "update" | "dedupe" | "outdated" | "list" | "ls" | "why"
            | "info" | "view" | "show" | "link" | "unlink" | "pm",
            ..,
        ] => Some("https://viteplus.dev/guide/install"),
        ["dev"] => Some("https://viteplus.dev/guide/dev"),
        ["check"] => Some("https://viteplus.dev/guide/check"),
        ["lint"] => Some("https://viteplus.dev/guide/lint"),
        ["fmt"] => Some("https://viteplus.dev/guide/fmt"),
        ["test"] => Some("https://viteplus.dev/guide/test"),
        ["run"] => Some("https://viteplus.dev/guide/run"),
        ["exec" | "dlx"] => Some("https://viteplus.dev/guide/vpx"),
        ["cache"] => Some("https://viteplus.dev/guide/cache"),
        ["build" | "preview"] => Some("https://viteplus.dev/guide/build"),
        ["pack"] => Some("https://viteplus.dev/guide/pack"),
        ["env", ..] => Some("https://viteplus.dev/guide/env"),
        ["upgrade"] => Some("https://viteplus.dev/guide/upgrade"),
        _ => None,
    }
}

pub fn render_heading(title: &str) -> String {
    let heading = format!("{title}:");
    if !should_style_help() {
        return heading;
    }

    if should_accent_heading(title) {
        heading.bold().bright_blue().to_string()
    } else {
        heading.bold().to_string()
    }
}

fn render_usage_value(usage: &str) -> String {
    if should_style_help() { usage.bold().to_string() } else { usage.to_string() }
}

fn should_accent_heading(title: &str) -> bool {
    title != "Usage"
}

fn write_documentation_footer(output: &mut String, documentation_url: &str) {
    let _ = writeln!(output);
    let _ = writeln!(output, "{} {documentation_url}", render_heading("Documentation"));
}

pub fn should_style_help() -> bool {
    std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("CLICOLOR").map_or(true, |value| value != "0")
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
}

fn render_rows(rows: &[HelpRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec![];
    }

    let label_width = rows.iter().map(|row| row.label.len()).max().unwrap_or(0);
    let mut output = Vec::new();

    for row in rows {
        let mut description_iter = row.description.iter();
        if let Some(first) = description_iter.next() {
            output.push(format!("  {:label_width$}  {}", row.label, first));
            for line in description_iter {
                output.push(format!("  {:label_width$}  {}", "", line));
            }
        } else {
            output.push(format!("  {}", row.label));
        }
    }

    output
}

fn render_owned_rows(rows: &[OwnedHelpRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec![];
    }

    let label_width = rows.iter().map(|row| row.label.chars().count()).max().unwrap_or(0);
    let mut output = Vec::new();

    for row in rows {
        let mut description_iter = row.description.iter();
        if let Some(first) = description_iter.next() {
            output.push(format!("  {:label_width$}  {}", row.label, first));
            for line in description_iter {
                output.push(format!("  {:label_width$}  {}", "", line));
            }
        } else {
            output.push(format!("  {}", row.label));
        }
    }

    output
}

fn split_comment_suffix(line: &str) -> Option<(&str, &str)> {
    line.find(" #").map(|index| line.split_at(index))
}

fn render_muted_comment_suffix(line: &str) -> String {
    if !should_style_help() {
        return line.to_string();
    }

    if let Some((prefix, suffix)) = split_comment_suffix(line) {
        return format!("{}{}", prefix, suffix.bright_black());
    }

    line.to_string()
}

pub fn render_help_doc(doc: &HelpDoc) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "{} {}", render_heading("Usage"), render_usage_value(doc.usage));

    if !doc.summary.is_empty() {
        let _ = writeln!(output);
        for line in &doc.summary {
            let _ = writeln!(output, "{line}");
        }
    }

    for section in &doc.sections {
        let _ = writeln!(output);
        match section {
            HelpSection::Rows { title, rows } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in render_rows(rows) {
                    let _ = writeln!(output, "{line}");
                }
            }
            HelpSection::Lines { title, lines } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in lines {
                    let _ = writeln!(output, "{}", render_muted_comment_suffix(line));
                }
            }
        }
    }

    if let Some(documentation_url) = doc.documentation_url {
        write_documentation_footer(&mut output, documentation_url);
    }

    output
}

fn render_owned_help_doc(doc: &OwnedHelpDoc) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "{} {}", render_heading("Usage"), render_usage_value(&doc.usage));

    if !doc.summary.is_empty() {
        let _ = writeln!(output);
        for line in &doc.summary {
            let _ = writeln!(output, "{line}");
        }
    }

    for section in &doc.sections {
        let _ = writeln!(output);
        match section {
            OwnedHelpSection::Rows { title, rows } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in render_owned_rows(rows) {
                    let _ = writeln!(output, "{line}");
                }
            }
            OwnedHelpSection::Lines { title, lines } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in lines {
                    let _ = writeln!(output, "{}", render_muted_comment_suffix(line));
                }
            }
        }
    }

    if let Some(documentation_url) = &doc.documentation_url {
        write_documentation_footer(&mut output, documentation_url);
    }

    output
}

fn is_section_heading(line: &str) -> bool {
    let trimmed = line.trim_end();
    !trimmed.is_empty() && !trimmed.starts_with(' ') && trimmed.ends_with(':')
}

fn split_label_and_description(content: &str) -> Option<(String, String)> {
    let bytes = content.as_bytes();
    let mut i = 0;

    while i + 1 < bytes.len() {
        if bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }

            let label = content[..i].trim_end();
            let description = content[j..].trim_start();
            if !label.is_empty() && !description.is_empty() {
                return Some((label.to_string(), description.to_string()));
            }
            i = j;
            continue;
        }
        i += 1;
    }

    None
}

fn parse_rows(lines: &[String]) -> Vec<OwnedHelpRow> {
    let mut rows = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let leading = line.chars().take_while(|c| *c == ' ').count();
        let content = line.trim_start();
        if content.is_empty() {
            continue;
        }

        if let Some((label, description)) = split_label_and_description(content) {
            rows.push(OwnedHelpRow { label, description: vec![description] });
            continue;
        }

        if leading >= 4 && content.starts_with('-') {
            rows.push(OwnedHelpRow { label: content.to_string(), description: vec![] });
            continue;
        }

        if leading >= 4 {
            if let Some(last) = rows.last_mut() {
                last.description.push(content.to_string());
                continue;
            }
        }

        rows.push(OwnedHelpRow { label: content.to_string(), description: vec![] });
    }

    rows
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek().copied() {
                // CSI sequence (for example: \x1b[1m)
                Some('[') => {
                    let _ = chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC sequence (for example: hyperlinks)
                Some(']') => {
                    let _ = chars.next();
                    let mut prev = '\0';
                    for c in chars.by_ref() {
                        if c == '\u{7}' || (prev == '\u{1b}' && c == '\\') {
                            break;
                        }
                        prev = c;
                    }
                }
                _ => {}
            }
            continue;
        }

        output.push(ch);
    }

    output
}

fn parse_clap_help_to_doc(raw_help: &str) -> Option<OwnedHelpDoc> {
    let normalized = raw_help.replace("\r\n", "\n");
    let lines: Vec<String> = normalized.lines().map(strip_ansi).collect();
    let usage_index = lines.iter().position(|line| line.starts_with("Usage: "))?;
    let usage = lines[usage_index].trim_start_matches("Usage: ").trim().to_string();

    let summary = lines[..usage_index]
        .iter()
        .map(|line| line.trim_end())
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut sections = Vec::new();
    let mut i = usage_index + 1;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }

        if !is_section_heading(&lines[i]) {
            i += 1;
            continue;
        }

        let title = lines[i].trim_end().trim_end_matches(':').to_string();
        i += 1;

        let mut body = Vec::new();
        while i < lines.len() {
            if is_section_heading(&lines[i]) {
                break;
            }
            body.push(lines[i].trim_end().to_string());
            i += 1;
        }

        let first_non_empty = body.iter().position(|line| !line.trim().is_empty());
        let last_non_empty = body.iter().rposition(|line| !line.trim().is_empty());
        let body = match (first_non_empty, last_non_empty) {
            (Some(start), Some(end)) if start <= end => body[start..=end].to_vec(),
            _ => vec![],
        };

        let row_sections =
            matches!(title.as_str(), "Arguments" | "Options" | "Commands" | "Subcommands");
        if row_sections {
            let rows = parse_rows(&body);
            sections.push(OwnedHelpSection::Rows { title, rows });
        } else {
            let lines = body.into_iter().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>();
            sections.push(OwnedHelpSection::Lines { title, lines });
        }
    }

    Some(OwnedHelpDoc { usage, summary, sections, documentation_url: None })
}

pub fn top_level_help_doc() -> HelpDoc {
    HelpDoc {
        usage: "vp [COMMAND]",
        summary: Vec::new(),
        sections: vec![
            section_rows(
                "Start",
                vec![
                    row("create", "Create a new project from a template"),
                    row("migrate", "Migrate an existing project to Vite+"),
                    row("config", "Configure hooks and agent integration"),
                    row("staged", "Run linters on staged files"),
                    row(
                        "install, i",
                        "Install all dependencies, or add packages if package names are provided",
                    ),
                    row("env", "Manage Node.js versions"),
                ],
            ),
            section_rows(
                "Develop",
                vec![
                    row("dev", "Run the development server"),
                    row("check", "Run format, lint, and type checks"),
                    row("lint", "Lint code"),
                    row("fmt", "Format code"),
                    row("test", "Run tests"),
                ],
            ),
            section_rows(
                "Execute",
                vec![
                    row("run", "Run tasks (also available as standalone `vpr`)"),
                    row("exec", "Execute a command from local node_modules/.bin"),
                    row("node", "Run a Node.js script (shorthand for `env exec node`)"),
                    row("dlx", "Execute a package binary without installing it as a dependency"),
                    row("cache", "Manage the task cache"),
                ],
            ),
            section_rows(
                "Build",
                vec![
                    row("build", "Build for production"),
                    row("pack", "Build library"),
                    row("preview", "Preview production build"),
                ],
            ),
            section_rows(
                "Manage Dependencies",
                vec![
                    row("add", "Add packages to dependencies"),
                    row("remove, rm, un, uninstall", "Remove packages from dependencies"),
                    row("update, up", "Update packages to their latest versions"),
                    row("dedupe", "Deduplicate dependencies by removing older versions"),
                    row("outdated", "Check for outdated packages"),
                    row("list, ls", "List installed packages"),
                    row("why, explain", "Show why a package is installed"),
                    row("info, view, show", "View package information from the registry"),
                    row("link, ln", "Link packages for local development"),
                    row("unlink", "Unlink packages"),
                    row("pm", "Forward a command to the package manager"),
                ],
            ),
            section_rows(
                "Maintain",
                vec![
                    row("upgrade", "Update vp itself to the latest version"),
                    row("implode", "Remove vp and all related data"),
                ],
            ),
        ],
        documentation_url: documentation_url_for_command_path(&[]),
    }
}

fn env_help_doc() -> HelpDoc {
    HelpDoc {
        usage: "vp env [COMMAND]",
        summary: vec!["Manage Node.js versions"],
        sections: vec![
            section_rows(
                "Setup",
                vec![
                    row("setup", "Create or update shims in VP_HOME/bin"),
                    row("on", "Enable managed mode - shims always use vite-plus managed Node.js"),
                    row(
                        "off",
                        "Enable system-first mode - shims prefer system Node.js, fallback to managed",
                    ),
                    row("print", "Print shell snippet to set environment for current session"),
                ],
            ),
            section_rows(
                "Manage",
                vec![
                    row("default", "Set or show the global default Node.js version"),
                    row(
                        "pin",
                        "Pin a Node.js version in the current directory (creates .node-version)",
                    ),
                    row(
                        "unpin",
                        "Remove the .node-version file from current directory (alias for `pin --unpin`)",
                    ),
                    row("use", "Use a specific Node.js version for this shell session"),
                    row("install", "Install a Node.js version [aliases: i]"),
                    row("uninstall", "Uninstall a Node.js version [aliases: uni]"),
                    row("exec", "Execute a command with a specific Node.js version [aliases: run]"),
                ],
            ),
            section_rows(
                "Inspect",
                vec![
                    row("current", "Show current environment information"),
                    row("doctor", "Run diagnostics and show environment status"),
                    row("which", "Show path to the tool that would be executed"),
                    row("list", "List locally installed Node.js versions [aliases: ls]"),
                    row(
                        "list-remote",
                        "List available Node.js versions from the registry [aliases: ls-remote]",
                    ),
                ],
            ),
            section_lines(
                "Examples",
                vec![
                    "  Setup:",
                    "    vp env setup                  # Create shims for node, npm, npx",
                    "    vp env on                     # Use vite-plus managed Node.js",
                    "    vp env print                  # Print shell snippet for this session",
                    "",
                    "  Manage:",
                    "    vp env pin lts                # Pin to latest LTS version",
                    "    vp env install                # Install version from .node-version / package.json",
                    "    vp env use 20                 # Use Node.js 20 for this shell session",
                    "    vp env use --unset            # Remove session override",
                    "",
                    "  Inspect:",
                    "    vp env current                # Show current resolved environment",
                    "    vp env current --json         # JSON output for automation",
                    "    vp env doctor                 # Check environment configuration",
                    "    vp env which node             # Show which node binary will be used",
                    "    vp env list-remote --lts      # List only LTS versions",
                    "",
                    "  Execute:",
                    "    vp env exec --node lts npm i  # Execute 'npm i' with latest LTS",
                    "    vp env exec node -v           # Shim mode (version auto-resolved)",
                ],
            ),
            section_lines(
                "Related Commands",
                vec![
                    "  vp install -g <package>       # Install a package globally",
                    "  vp uninstall -g <package>     # Uninstall a package globally",
                    "  vp update -g [package]        # Update global packages",
                    "  vp list -g [package]          # List global packages",
                ],
            ),
        ],
        documentation_url: documentation_url_for_command_path(&["env"]),
    }
}

fn delegated_help_doc(command: &str) -> Option<HelpDoc> {
    match command {
        "dev" => Some(HelpDoc {
            usage: "vp dev [ROOT] [OPTIONS]",
            summary: vec!["Run the development server.", "Options are forwarded to Vite."],
            sections: vec![
                section_rows(
                    "Arguments",
                    vec![row("[ROOT]", "Project root directory (default: current directory)")],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("--host [HOST]", "Specify hostname"),
                        row("--port <PORT>", "Specify port"),
                        row("--open [PATH]", "Open browser on startup"),
                        row("--strictPort", "Exit if specified port is already in use"),
                        row("-c, --config <FILE>", "Use specified config file"),
                        row("--base <PATH>", "Public base path"),
                        row("-m, --mode <MODE>", "Set env mode"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec!["  vp dev", "  vp dev --open", "  vp dev --host localhost --port 5173"],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["dev"]),
        }),
        "build" => Some(HelpDoc {
            usage: "vp build [ROOT] [OPTIONS]",
            summary: vec!["Build for production.", "Options are forwarded to Vite."],
            sections: vec![
                section_rows(
                    "Arguments",
                    vec![row("[ROOT]", "Project root directory (default: current directory)")],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("--target <TARGET>", "Transpile target"),
                        row("--outDir <DIR>", "Output directory"),
                        row("--sourcemap [MODE]", "Output source maps"),
                        row("--minify [MINIFIER]", "Enable/disable minification"),
                        row("-w, --watch", "Rebuild when files change"),
                        row("-c, --config <FILE>", "Use specified config file"),
                        row("-m, --mode <MODE>", "Set env mode"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec!["  vp build", "  vp build --watch", "  vp build --sourcemap"],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["build"]),
        }),
        "preview" => Some(HelpDoc {
            usage: "vp preview [ROOT] [OPTIONS]",
            summary: vec!["Preview production build.", "Options are forwarded to Vite."],
            sections: vec![
                section_rows(
                    "Arguments",
                    vec![row("[ROOT]", "Project root directory (default: current directory)")],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("--host [HOST]", "Specify hostname"),
                        row("--port <PORT>", "Specify port"),
                        row("--strictPort", "Exit if specified port is already in use"),
                        row("--open [PATH]", "Open browser on startup"),
                        row("--outDir <DIR>", "Output directory to preview"),
                        row("-c, --config <FILE>", "Use specified config file"),
                        row("-m, --mode <MODE>", "Set env mode"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines("Examples", vec!["  vp preview", "  vp preview --port 4173"]),
            ],
            documentation_url: documentation_url_for_command_path(&["preview"]),
        }),
        "test" => Some(HelpDoc {
            usage: "vp test [COMMAND] [FILTERS] [OPTIONS]",
            summary: vec!["Run tests.", "Options are forwarded to Vitest."],
            sections: vec![
                section_rows(
                    "Commands",
                    vec![
                        row("run", "Run tests once"),
                        row("watch", "Run tests in watch mode"),
                        row("dev", "Run tests in development mode"),
                        row("related", "Run tests related to changed files"),
                        row("bench", "Run benchmarks"),
                        row("init", "Initialize Vitest config"),
                        row("list", "List matching tests"),
                    ],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("-c, --config <PATH>", "Path to config file"),
                        row("-w, --watch", "Enable watch mode"),
                        row("-t, --testNamePattern <PATTERN>", "Run tests matching regexp"),
                        row("--ui", "Enable UI"),
                        row("--coverage", "Enable coverage"),
                        row("--reporter <NAME>", "Specify reporter"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec![
                        "  vp test",
                        "  vp test run src/foo.test.ts",
                        "  vp test watch --coverage",
                    ],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["test"]),
        }),
        "lint" => Some(HelpDoc {
            usage: "vp lint [PATH]... [OPTIONS]",
            summary: vec!["Lint code.", "Options are forwarded to Oxlint."],
            sections: vec![
                section_rows(
                    "Options",
                    vec![
                        row("--tsconfig <PATH>", "TypeScript tsconfig path"),
                        row("--fix", "Fix issues when possible"),
                        row("--type-aware", "Enable rules requiring type information"),
                        row("--import-plugin", "Enable import plugin"),
                        row("--rules", "List registered rules"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec![
                        "  vp lint",
                        "  vp lint src --fix",
                        "  vp lint --type-aware --tsconfig ./tsconfig.json",
                    ],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["lint"]),
        }),
        "fmt" => Some(HelpDoc {
            usage: "vp fmt [PATH]... [OPTIONS]",
            summary: vec!["Format code.", "Options are forwarded to Oxfmt."],
            sections: vec![
                section_rows(
                    "Options",
                    vec![
                        row("--write", "Format and write files in place"),
                        row("--check", "Check if files are formatted"),
                        row("--list-different", "List files that would be changed"),
                        row("--ignore-path <PATH>", "Path to ignore file(s)"),
                        row("--threads <INT>", "Number of threads to use"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec!["  vp fmt", "  vp fmt src --check", "  vp fmt . --write"],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["fmt"]),
        }),
        "check" => Some(HelpDoc {
            usage: "vp check [OPTIONS] [PATHS]...",
            summary: vec!["Run format, lint, and type checks."],
            sections: vec![
                section_rows(
                    "Options",
                    vec![
                        row("--fix", "Auto-fix format and lint issues"),
                        row("--no-fmt", "Skip format check"),
                        row("--no-lint", "Skip lint check"),
                        row(
                            "--no-error-on-unmatched-pattern",
                            "Do not exit with error when pattern is unmatched",
                        ),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec!["  vp check", "  vp check --fix", "  vp check --no-lint src/index.ts"],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["check"]),
        }),
        "pack" => Some(HelpDoc {
            usage: "vp pack [...FILES] [OPTIONS]",
            summary: vec!["Build library.", "Options are forwarded to tsdown."],
            sections: vec![
                section_rows(
                    "Options",
                    vec![
                        row("-f, --format <FORMAT>", "Bundle format: esm, cjs, iife, umd"),
                        row("-d, --out-dir <DIR>", "Output directory"),
                        row("--sourcemap", "Generate source map"),
                        row("--dts", "Generate dts files"),
                        row("--minify", "Minify output"),
                        row("-w, --watch [PATH]", "Watch mode"),
                        row("-h, --help", "Print help"),
                    ],
                ),
                section_lines(
                    "Examples",
                    vec!["  vp pack", "  vp pack src/index.ts --dts", "  vp pack --watch"],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["pack"]),
        }),
        "run" => Some(HelpDoc {
            usage: "vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...",
            summary: vec!["Run tasks."],
            sections: vec![
                section_rows(
                    "Arguments",
                    vec![
                        row(
                            "[TASK_SPECIFIER]",
                            "`packageName#taskName` or `taskName`. If omitted, lists all available tasks",
                        ),
                        row("[ADDITIONAL_ARGS]...", "Additional arguments to pass to the tasks"),
                    ],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("-r, --recursive", "Select all packages in the workspace"),
                        row(
                            "-t, --transitive",
                            "Select the current package and its transitive dependencies",
                        ),
                        row("-w, --workspace-root", "Select the workspace root package"),
                        row(
                            "-F, --filter <FILTERS>",
                            "Match packages by name, directory, or glob pattern",
                        ),
                        row(
                            "--ignore-depends-on",
                            "Do not run dependencies specified in `dependsOn` fields",
                        ),
                        row("-v, --verbose", "Show full detailed summary after execution"),
                        row("--last-details", "Display the detailed summary of the last run"),
                        row("-h, --help", "Print help (see more with '--help')"),
                    ],
                ),
                section_lines(
                    "Filter Patterns",
                    vec![
                        "  --filter <pattern>        Select by package name (e.g. foo, @scope/*)",
                        "  --filter ./<dir>          Select packages under a directory",
                        "  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes",
                        "  --filter <pattern>...     Select package and its dependencies",
                        "  --filter ...<pattern>     Select package and its dependents",
                        "  --filter <pattern>^...    Select only the dependencies (exclude the package itself)",
                        "  --filter !<pattern>       Exclude packages matching the pattern",
                    ],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["run"]),
        }),
        "exec" => Some(HelpDoc {
            usage: "vp exec [OPTIONS] [COMMAND]...",
            summary: vec!["Execute a command from local node_modules/.bin."],
            sections: vec![
                section_rows(
                    "Arguments",
                    vec![row("[COMMAND]...", "Command and arguments to execute")],
                ),
                section_rows(
                    "Options",
                    vec![
                        row("-r, --recursive", "Select all packages in the workspace"),
                        row(
                            "-t, --transitive",
                            "Select the current package and its transitive dependencies",
                        ),
                        row("-w, --workspace-root", "Select the workspace root package"),
                        row(
                            "-F, --filter <FILTERS>",
                            "Match packages by name, directory, or glob pattern",
                        ),
                        row("-c, --shell-mode", "Execute the command within a shell environment"),
                        row("--parallel", "Run concurrently without topological ordering"),
                        row("--reverse", "Reverse execution order"),
                        row("--resume-from <RESUME_FROM>", "Resume from a specific package"),
                        row("--report-summary", "Save results to vp-exec-summary.json"),
                        row("-h, --help", "Print help (see more with '--help')"),
                    ],
                ),
                section_lines(
                    "Filter Patterns",
                    vec![
                        "  --filter <pattern>        Select by package name (e.g. foo, @scope/*)",
                        "  --filter ./<dir>          Select packages under a directory",
                        "  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes",
                        "  --filter <pattern>...     Select package and its dependencies",
                        "  --filter ...<pattern>     Select package and its dependents",
                        "  --filter <pattern>^...    Select only the dependencies (exclude the package itself)",
                        "  --filter !<pattern>       Exclude packages matching the pattern",
                    ],
                ),
                section_lines(
                    "Examples",
                    vec![
                        "  vp exec node --version                             # Run local node",
                        "  vp exec tsc --noEmit                               # Run local TypeScript compiler",
                        "  vp exec -c 'tsc --noEmit && prettier --check .'    # Shell mode",
                        "  vp exec -r -- tsc --noEmit                         # Run in all workspace packages",
                        "  vp exec --filter 'app...' -- tsc                   # Run in filtered packages",
                    ],
                ),
            ],
            documentation_url: documentation_url_for_command_path(&["exec"]),
        }),
        "cache" => Some(HelpDoc {
            usage: "vp cache <COMMAND>",
            summary: vec!["Manage the task cache."],
            sections: vec![
                section_rows("Commands", vec![row("clean", "Clean up all the cache")]),
                section_rows("Options", vec![row("-h, --help", "Print help")]),
            ],
            documentation_url: documentation_url_for_command_path(&["cache"]),
        }),
        _ => None,
    }
}

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn has_help_flag_before_terminator(args: &[String]) -> bool {
    args.iter().take_while(|arg| arg.as_str() != "--").any(|arg| is_help_flag(arg))
}

fn skip_clap_unified_help(command: &str) -> bool {
    matches!(
        command,
        "create"
            | "migrate"
            | "dev"
            | "build"
            | "preview"
            | "test"
            | "lint"
            | "fmt"
            | "check"
            | "pack"
            | "run"
            | "exec"
            | "cache"
    )
}

pub fn maybe_print_unified_clap_subcommand_help(argv: &[String]) -> bool {
    if argv.len() < 3 {
        return false;
    }

    let command = crate::cli::Args::command();
    let mut current = &command;
    let mut path_len = 0;
    let mut index = 1;
    let mut first_command_name: Option<String> = None;
    let mut command_path = Vec::new();

    while index < argv.len() {
        let arg = &argv[index];
        if arg.starts_with('-') {
            break;
        }

        let Some(next) = current.find_subcommand(arg) else {
            break;
        };

        if first_command_name.is_none() {
            first_command_name = Some(next.get_name().to_string());
        }

        command_path.push(next.get_name().to_string());
        current = next;
        path_len += 1;
        index += 1;
    }

    if path_len == 0 {
        return false;
    }

    let Some(first_command_name) = first_command_name else {
        return false;
    };
    if skip_clap_unified_help(&first_command_name) {
        return false;
    }

    // Respect `--` option terminator: flags after `--` belong to the wrapped
    // command and should not trigger CLI help rewriting.
    if !has_help_flag_before_terminator(&argv[index..]) {
        return false;
    }

    if command_path.len() == 1 && command_path[0] == "env" {
        println!("{}", vite_shared::header::vite_plus_header());
        println!();
        println!("{}", render_help_doc(&env_help_doc()));
        return true;
    }

    let mut command_path_refs = Vec::with_capacity(command_path.len());
    for segment in &command_path {
        command_path_refs.push(segment.as_str());
    }
    print_unified_clap_help_for_path(&command_path_refs)
}

pub fn should_print_unified_delegate_help(args: &[String]) -> bool {
    matches!(args, [arg] if is_help_flag(arg))
}

pub fn maybe_print_unified_delegate_help(
    command: &str,
    args: &[String],
    show_header: bool,
) -> bool {
    if !should_print_unified_delegate_help(args) {
        return false;
    }

    let Some(doc) = delegated_help_doc(command) else {
        return false;
    };

    if show_header {
        println!("{}", vite_shared::header::vite_plus_header());
        println!();
    }
    println!("{}", render_help_doc(&doc));
    true
}

pub fn print_unified_clap_help_for_path(command_path: &[&str]) -> bool {
    if command_path == ["env"] {
        println!("{}", vite_shared::header::vite_plus_header());
        println!();
        println!("{}", render_help_doc(&env_help_doc()));
        return true;
    }

    let mut help_args = vec!["vp".to_string()];
    help_args.extend(command_path.iter().map(ToString::to_string));
    help_args.push("--help".to_string());

    let raw_help = match crate::cli::try_parse_args_from(help_args) {
        Err(error) if matches!(error.kind(), ErrorKind::DisplayHelp) => error.to_string(),
        _ => return false,
    };

    let Some(doc) = parse_clap_help_to_doc(&raw_help) else {
        return false;
    };
    let doc = OwnedHelpDoc {
        documentation_url: documentation_url_for_command_path(command_path)
            .map(ToString::to_string),
        ..doc
    };

    println!("{}", vite_shared::header::vite_plus_header());
    println!();
    println!("{}", render_owned_help_doc(&doc));
    true
}

#[cfg(test)]
mod tests {
    use super::{
        HelpDoc, documentation_url_for_command_path, has_help_flag_before_terminator,
        parse_clap_help_to_doc, parse_rows, render_help_doc, split_comment_suffix, strip_ansi,
    };

    #[test]
    fn parse_rows_supports_wrapped_option_labels() {
        let lines = vec![
            "  -P, --prod            Do not install devDependencies".to_string(),
            "  --no-optional".to_string(),
            "                        Do not install optionalDependencies".to_string(),
        ];

        let rows = parse_rows(&lines);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "-P, --prod");
        assert_eq!(rows[0].description, vec!["Do not install devDependencies"]);
        assert_eq!(rows[1].label, "--no-optional");
        assert_eq!(rows[1].description, vec!["Do not install optionalDependencies"]);
    }

    #[test]
    fn parse_clap_help_extracts_usage_summary_and_sections() {
        let raw_help = "\
Add packages to dependencies

Usage: vp add [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to add

Options:
  -h, --help  Print help
";

        let doc = parse_clap_help_to_doc(raw_help).expect("should parse clap help text");
        assert_eq!(doc.usage, "vp add [OPTIONS] <PACKAGES>...");
        assert_eq!(doc.summary, vec!["Add packages to dependencies"]);
        assert_eq!(doc.sections.len(), 2);
    }

    #[test]
    fn help_flag_before_terminator_is_detected() {
        let args = vec!["vpx".to_string(), "--help".to_string()];
        assert!(has_help_flag_before_terminator(&args));
    }

    #[test]
    fn help_flag_after_terminator_is_ignored() {
        let args = vec!["vpx".to_string(), "--".to_string(), "--help".to_string()];
        assert!(!has_help_flag_before_terminator(&args));
    }

    #[test]
    fn strip_ansi_removes_csi_sequences() {
        let input = "\u{1b}[1mOptions:\u{1b}[0m";
        assert_eq!(strip_ansi(input), "Options:");
    }

    #[test]
    fn parse_clap_help_with_ansi_sequences() {
        let raw_help = "\
\u{1b}[1mAdd packages to dependencies\u{1b}[0m

\u{1b}[1mUsage:\u{1b}[0m vp add [OPTIONS] <PACKAGES>...

\u{1b}[1mArguments:\u{1b}[0m
  <PACKAGES>...  Packages to add

\u{1b}[1mOptions:\u{1b}[0m
  -h, --help  Print help
";

        let doc = parse_clap_help_to_doc(raw_help).expect("should parse clap help text");
        assert_eq!(doc.usage, "vp add [OPTIONS] <PACKAGES>...");
        assert_eq!(doc.summary, vec!["Add packages to dependencies"]);
        assert_eq!(doc.sections.len(), 2);
    }

    #[test]
    fn split_comment_suffix_extracts_command_comment() {
        let line = "  vp env list-remote 20         # List Node.js 20.x versions";
        let (prefix, suffix) = split_comment_suffix(line).expect("expected comment suffix");
        assert_eq!(prefix, "  vp env list-remote 20        ");
        assert_eq!(suffix, " # List Node.js 20.x versions");
    }

    #[test]
    fn split_comment_suffix_returns_none_without_comment() {
        assert!(split_comment_suffix("  vp env list").is_none());
    }

    #[test]
    fn docs_url_is_mapped_for_grouped_commands() {
        assert_eq!(
            documentation_url_for_command_path(&["add"]),
            Some("https://viteplus.dev/guide/install")
        );
        assert_eq!(
            documentation_url_for_command_path(&["env", "list"]),
            Some("https://viteplus.dev/guide/env")
        );
        assert_eq!(
            documentation_url_for_command_path(&["config"]),
            Some("https://viteplus.dev/guide/commit-hooks")
        );
    }

    #[test]
    fn render_help_doc_appends_documentation_footer() {
        let output = render_help_doc(&HelpDoc {
            usage: "vp demo",
            summary: vec![],
            sections: vec![],
            documentation_url: Some("https://viteplus.dev/guide/demo"),
        });

        assert!(output.contains("Documentation: https://viteplus.dev/guide/demo"));
    }
}
