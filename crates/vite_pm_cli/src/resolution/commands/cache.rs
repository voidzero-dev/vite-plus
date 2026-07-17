use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct CacheArgs {
    /// Subcommand: dir, path, clean
    #[arg(required = true)]
    pub(crate) subcommand: String,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<CacheArgs> for Pnpm {
    fn resolve(&self, args: &CacheArgs, diag: &mut Diagnostics) -> CommandResolution {
        match args.subcommand.as_str() {
            "dir" | "path" => resolve_cache("pnpm", &["store", "path"], args),
            "clean" => resolve_cache("pnpm", &["store", "prune"], args),
            subcommand => unsupported("pnpm cache", subcommand, diag),
        }
    }
}

impl Resolve<CacheArgs> for Npm {
    fn resolve(&self, args: &CacheArgs, diag: &mut Diagnostics) -> CommandResolution {
        match args.subcommand.as_str() {
            "dir" | "path" => resolve_cache("npm", &["config", "get", "cache"], args),
            "clean" => resolve_cache("npm", &["cache", "clean"], args),
            subcommand => unsupported("npm cache", subcommand, diag),
        }
    }
}

impl Resolve<CacheArgs> for Yarn {
    fn resolve(&self, args: &CacheArgs, diag: &mut Diagnostics) -> CommandResolution {
        match args.subcommand.as_str() {
            "dir" | "path" if self.is_berry() => {
                resolve_cache("yarn", &["config", "get", "cacheFolder"], args)
            }
            "dir" | "path" => resolve_cache("yarn", &["cache", "dir"], args),
            "clean" => resolve_cache("yarn", &["cache", "clean"], args),
            subcommand => unsupported("yarn cache", subcommand, diag),
        }
    }
}

impl Resolve<CacheArgs> for Bun {
    fn resolve(&self, args: &CacheArgs, diag: &mut Diagnostics) -> CommandResolution {
        match args.subcommand.as_str() {
            "dir" | "path" => resolve_cache("bun", &["pm", "cache"], args),
            "clean" => resolve_cache("bun", &["pm", "cache", "rm"], args),
            subcommand => unsupported("bun pm cache", subcommand, diag),
        }
    }
}

fn resolve_cache(program: &str, command_args: &[&str], args: &CacheArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    for arg in command_args {
        cmd.arg(arg);
    }
    cmd.extend(args.pass_through_args.iter());
    cmd.into()
}

fn unsupported(command: &str, subcommand: &str, diag: &mut Diagnostics) -> CommandResolution {
    diag.warn(
        DiagnosticKind::UnsupportedCommandNoop,
        vite_str::format!("{command} subcommand '{subcommand}' not supported"),
    );
    CommandResolution::Noop
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn cache(subcommand: &str) -> CacheArgs {
        CacheArgs { subcommand: subcommand.to_string(), ..Default::default() }
    }

    #[test]
    fn test_parser_accepts_subcommand_and_pass_through_args() {
        let args = parse_args::<CacheArgs>(["clean", "--", "--force"]).unwrap();

        assert_eq!(args.subcommand, "clean");
        assert_eq!(args.pass_through_args, vec!["--force".to_string()]);
    }

    #[test]
    fn test_pnpm_cache_dir() {
        let resolution = resolve(&pnpm("10.0.0"), cache("dir"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["store", "path"]);
    }

    #[test]
    fn test_npm_cache_dir() {
        let resolution = resolve(&npm("11.0.0"), cache("dir"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["config", "get", "cache"]);
    }

    #[test]
    fn test_yarn1_cache_dir() {
        let resolution = resolve(&yarn("1.22.0"), cache("dir"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["cache", "dir"]);
    }

    #[test]
    fn test_yarn2_cache_dir() {
        let resolution = resolve(&yarn("4.0.0"), cache("dir"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "get", "cacheFolder"]);
    }

    #[test]
    fn test_pnpm_cache_clean() {
        let resolution = resolve(&pnpm("10.0.0"), cache("clean"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["store", "prune"]);
    }

    #[test]
    fn test_npm_cache_clean() {
        let resolution = resolve(&npm("11.0.0"), cache("clean"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_yarn1_cache_clean() {
        let resolution = resolve(&yarn("1.22.0"), cache("clean"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_yarn2_cache_clean() {
        let resolution = resolve(&yarn("4.0.0"), cache("clean"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_bun_cache_dir() {
        let resolution = resolve(&bun("1.3.11"), cache("path"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "cache"]);
    }

    #[test]
    fn test_bun_cache_clean() {
        let resolution = resolve(&bun("1.3.11"), cache("clean"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "cache", "rm"]);
    }

    #[test]
    fn test_unsupported_cache_subcommand_noops() {
        let resolution = resolve(&pnpm("10.0.0"), cache("verify"));

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(
            resolution.diagnostics[0].message,
            "pnpm cache subcommand 'verify' not supported"
        );
    }

    #[test]
    fn test_cache_pass_through_args() {
        let resolution = resolve(
            &npm("11.0.0"),
            CacheArgs {
                subcommand: "clean".to_string(),
                pass_through_args: vec!["--force".to_string()],
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["cache", "clean", "--force"]);
    }
}
