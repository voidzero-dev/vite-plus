use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

/// Configuration subcommands.
#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum ConfigCommand {
    /// List all configuration
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Get configuration value
    Get {
        /// Config key
        key: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Set configuration value
    Set {
        /// Config key
        key: String,

        /// Config value
        value: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Delete configuration key
    Delete {
        /// Config key
        key: String,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },
}

impl Resolve<ConfigCommand> for Pnpm {
    fn resolve(&self, args: &ConfigCommand, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_npm_like_config("pnpm", args)
    }
}

impl Resolve<ConfigCommand> for Npm {
    fn resolve(&self, args: &ConfigCommand, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_npm_like_config("npm", args)
    }
}

impl Resolve<ConfigCommand> for Yarn {
    fn resolve(&self, args: &ConfigCommand, diag: &mut Diagnostics) -> CommandResolution {
        resolve_yarn_config(args, self.is_berry(), diag)
    }
}

impl Resolve<ConfigCommand> for Bun {
    fn resolve(&self, args: &ConfigCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun uses bunfig.toml for configuration, not a config command. Falling back to npm config.",
        );
        resolve_npm_like_config("bun", args)
    }
}

fn resolve_npm_like_config(program: &str, args: &ConfigCommand) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("config").arg(args.subcommand_name());
    append_key_value(&mut cmd, args);
    cmd.arg_if("--json", args.json());
    if let Some(location) = args.effective_location() {
        cmd.arg("--location").arg(location);
    }
    cmd.into()
}

fn resolve_yarn_config(
    args: &ConfigCommand,
    is_berry: bool,
    diag: &mut Diagnostics,
) -> CommandResolution {
    let mut cmd = CommandBuilder::new("yarn");
    cmd.arg("config");
    match (args, is_berry) {
        (ConfigCommand::Delete { .. }, true) => {
            cmd.arg("unset");
        }
        (ConfigCommand::List { .. }, true) => {}
        _ => {
            cmd.arg(args.subcommand_name());
        }
    }
    append_key_value(&mut cmd, args);
    cmd.arg_if("--json", args.json());
    if let Some(location) = args.effective_location() {
        if is_berry {
            if location == "global" {
                cmd.arg("--home");
            }
        } else if location == "global" {
            cmd.arg("--global");
        } else {
            diag.warn(
                DiagnosticKind::UnsupportedOptionDropped,
                "yarn@1 does not support --location, ignoring flag",
            );
        }
    }
    cmd.into()
}

fn append_key_value(cmd: &mut CommandBuilder, command: &ConfigCommand) {
    if let Some(key) = command.key() {
        cmd.arg(key);
    }
    if let Some(value) = command.value() {
        cmd.arg(value);
    }
}

impl ConfigCommand {
    fn subcommand_name(&self) -> &'static str {
        match self {
            Self::List { .. } => "list",
            Self::Get { .. } => "get",
            Self::Set { .. } => "set",
            Self::Delete { .. } => "delete",
        }
    }

    fn key(&self) -> Option<&str> {
        match self {
            Self::List { .. } => None,
            Self::Get { key, .. } | Self::Set { key, .. } | Self::Delete { key, .. } => Some(key),
        }
    }

    fn value(&self) -> Option<&str> {
        match self {
            Self::Set { value, .. } => Some(value),
            Self::List { .. } | Self::Get { .. } | Self::Delete { .. } => None,
        }
    }

    fn json(&self) -> bool {
        match self {
            Self::List { json, .. } | Self::Get { json, .. } | Self::Set { json, .. } => *json,
            Self::Delete { .. } => false,
        }
    }

    fn effective_location(&self) -> Option<&str> {
        match self {
            Self::List { global, location, .. }
            | Self::Get { global, location, .. }
            | Self::Set { global, location, .. }
            | Self::Delete { global, location, .. } => {
                if *global {
                    Some("global")
                } else {
                    location.as_deref()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        Resolution, resolve,
        test_utils::{bun, npm, parse_subcommand, pnpm, yarn},
    };

    fn set_config(location: Option<&str>) -> ConfigCommand {
        ConfigCommand::Set {
            key: "registry".to_string(),
            value: "https://registry.npmjs.org".to_string(),
            json: false,
            global: false,
            location: location.map(ToString::to_string),
        }
    }

    #[test]
    fn test_parser_accepts_global_short_flag() {
        let args = parse_subcommand::<ConfigCommand>(["get", "registry", "-g"]).unwrap();

        assert_eq!(
            args,
            ConfigCommand::Get {
                key: "registry".to_string(),
                json: false,
                global: true,
                location: None,
            }
        );
    }

    #[test]
    fn test_pnpm_config_set() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&pnpm("10.0.0"), set_config(None))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_npm_config_set() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&npm("11.0.0"), set_config(None))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_config_set_with_json() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            ConfigCommand::Set {
                key: "registry".to_string(),
                value: "https://registry.npmjs.org".to_string(),
                json: true,
                global: false,
                location: None,
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--json"]
        );
    }

    #[test]
    fn test_config_set_with_location_global() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&pnpm("10.0.0"), set_config(Some("global")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn2_config_set_location_global() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&yarn("4.0.0"), set_config(Some("global")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--home"]
        );
    }

    #[test]
    fn test_yarn1_config_set() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&yarn("1.22.0"), set_config(None))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_pnpm_config_set_global() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            ConfigCommand::Set {
                key: "registry".to_string(),
                value: "https://registry.npmjs.org".to_string(),
                json: false,
                global: true,
                location: None,
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_npm_config_set_global() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&npm("11.0.0"), set_config(Some("global")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn1_config_set_global() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&yarn("1.22.0"), set_config(Some("global")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--global"]
        );
    }

    #[test]
    fn test_pnpm_config_get() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            ConfigCommand::Get {
                key: "registry".to_string(),
                json: false,
                global: false,
                location: None,
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["config", "get", "registry"]);
    }

    #[test]
    fn test_npm_config_delete() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            ConfigCommand::Delete { key: "registry".to_string(), global: false, location: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["config", "delete", "registry"]);
    }

    #[test]
    fn test_yarn2_config_delete() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &yarn("4.0.0"),
            ConfigCommand::Delete { key: "registry".to_string(), global: false, location: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "unset", "registry"]);
    }

    #[test]
    fn test_yarn2_config_list() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &yarn("4.0.0"),
            ConfigCommand::List { json: false, global: false, location: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config"]);
    }

    #[test]
    fn test_yarn1_location_project_warns_and_drops() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } =
            resolve(&yarn("1.22.0"), set_config(Some("project")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
        assert_eq!(diagnostics[0].message, "yarn@1 does not support --location, ignoring flag");
    }

    #[test]
    fn test_yarn2_location_project_is_silently_ignored() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } =
            resolve(&yarn("4.0.0"), set_config(Some("project")))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_bun_config_fallback_keeps_bun_program() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } =
            resolve(&bun("1.3.11"), set_config(None))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
        assert_eq!(
            diagnostics[0].message,
            "bun uses bunfig.toml for configuration, not a config command. Falling back to npm config."
        );
    }
}
