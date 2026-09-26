use vp_pm_cli_macros::pm_args;

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

        /// Config location: project, user, or global
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
    fn resolve(&self, args: &ConfigCommand, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_yarn_config(args, self.is_berry())
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

fn resolve_yarn_config(args: &ConfigCommand, is_berry: bool) -> CommandResolution {
    if let Some(location) = args.effective_location() {
        // Classic's set/delete use saveHomeConfig, so user scope needs no flag.
        // https://github.com/yarnpkg/yarn/blob/v1.22.22/src/cli/commands/config.js#L50-L80
        let supported =
            matches!(location, "user" | "global") || (is_berry && location == "project");
        if !supported {
            let manager = if is_berry { "yarn >= 2" } else { "Yarn Classic" };
            return CommandResolution::InvalidArgument(
                vt_str::format!("{manager} does not support --location {location}.").to_string(),
            );
        }
    }
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
    // Reads keep the merged configuration, as npm and pnpm do for user scope.
    // Berry only accepts `--home` on `config set` and `config unset`.
    // https://yarnpkg.com/cli/config/set
    // https://yarnpkg.com/cli/config/get
    let is_write = matches!(args, ConfigCommand::Set { .. } | ConfigCommand::Delete { .. });
    match args.effective_location() {
        Some("global") if !is_berry => {
            cmd.arg("--global");
        }
        Some("user" | "global") if is_berry && is_write => {
            cmd.arg("--home");
        }
        _ => {}
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
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_subcommand, pnpm, yarn},
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
        let command = expect_run(resolve(&pnpm("10.0.0"), set_config(None)).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_npm_config_set() {
        let command = expect_run(resolve(&npm("11.0.0"), set_config(None)).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_config_set_with_json() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                ConfigCommand::Set {
                    key: "registry".to_string(),
                    value: "https://registry.npmjs.org".to_string(),
                    json: true,
                    global: false,
                    location: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--json"]
        );
    }

    #[test]
    fn test_config_set_with_location_global() {
        let command = expect_run(resolve(&pnpm("10.0.0"), set_config(Some("global"))).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn2_config_set_location_global() {
        let command = expect_run(resolve(&yarn("4.0.0"), set_config(Some("global"))).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--home"]
        );
    }

    #[test]
    fn test_yarn1_config_set() {
        let command = expect_run(resolve(&yarn("1.22.0"), set_config(None)).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_pnpm_config_set_global() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                ConfigCommand::Set {
                    key: "registry".to_string(),
                    value: "https://registry.npmjs.org".to_string(),
                    json: false,
                    global: true,
                    location: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_npm_config_set_global() {
        let command = expect_run(resolve(&npm("11.0.0"), set_config(Some("global"))).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn1_config_set_global() {
        let command = expect_run(resolve(&yarn("1.22.0"), set_config(Some("global"))).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--global"]
        );
    }

    #[test]
    fn test_pnpm_config_get() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                ConfigCommand::Get {
                    key: "registry".to_string(),
                    json: false,
                    global: false,
                    location: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["config", "get", "registry"]);
    }

    #[test]
    fn test_npm_config_delete() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                ConfigCommand::Delete {
                    key: "registry".to_string(),
                    global: false,
                    location: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["config", "delete", "registry"]);
    }

    #[test]
    fn test_yarn2_config_delete() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                ConfigCommand::Delete {
                    key: "registry".to_string(),
                    global: false,
                    location: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config", "unset", "registry"]);
    }

    #[test]
    fn test_yarn2_config_list() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                ConfigCommand::List { json: false, global: false, location: None },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["config"]);
    }

    #[test]
    fn yarn_rejects_unsupported_locations() {
        for (version, location, message) in [
            ("1.22.22", "project", "Yarn Classic does not support --location project."),
            ("4.18.0", "unknown", "yarn >= 2 does not support --location unknown."),
        ] {
            expect_unsupported(resolve(&yarn(version), set_config(Some(location))), &[message]);
        }
    }

    #[test]
    fn yarn_maps_supported_locations() {
        let classic = resolve(&yarn("1.22.22"), set_config(Some("user")));
        assert_eq!(classic.outcome, resolve(&yarn("1.22.22"), set_config(None)).outcome);
        assert!(classic.diagnostics.is_empty());

        let project = resolve(&yarn("4.18.0"), set_config(Some("project")));
        assert_eq!(project.outcome, resolve(&yarn("4.18.0"), set_config(None)).outcome);
        assert!(project.diagnostics.is_empty());

        let user_set = resolve(&yarn("4.18.0"), set_config(Some("user")));
        assert_eq!(
            expect_run(user_set.outcome).args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--home"]
        );
        assert!(user_set.diagnostics.is_empty());

        let user_get = resolve(
            &yarn("4.18.0"),
            ConfigCommand::Get {
                key: "registry".to_string(),
                json: false,
                global: false,
                location: Some("user".to_string()),
            },
        );
        assert_eq!(expect_run(user_get.outcome).args, vec!["config", "get", "registry"]);
        assert!(user_get.diagnostics.is_empty());

        // `yarn config get` and `yarn config` have no `--home`; reads stay merged.
        let global_get = resolve(
            &yarn("4.18.0"),
            ConfigCommand::Get {
                key: "registry".to_string(),
                json: false,
                global: true,
                location: None,
            },
        );
        assert_eq!(expect_run(global_get.outcome).args, vec!["config", "get", "registry"]);
        assert!(global_get.diagnostics.is_empty());
        let global_list = resolve(
            &yarn("4.18.0"),
            ConfigCommand::List { json: false, global: true, location: None },
        );
        assert_eq!(expect_run(global_list.outcome).args, vec!["config"]);
        assert!(global_list.diagnostics.is_empty());

        let user_delete = resolve(
            &yarn("4.18.0"),
            ConfigCommand::Delete {
                key: "registry".to_string(),
                global: false,
                location: Some("user".to_string()),
            },
        );
        assert_eq!(
            expect_run(user_delete.outcome).args,
            vec!["config", "unset", "registry", "--home"]
        );
        assert!(user_delete.diagnostics.is_empty());
    }

    #[test]
    fn yarn_global_keeps_precedence_over_location() {
        for version in ["1.22.22", "4.18.0"] {
            let args = parse_subcommand::<ConfigCommand>([
                "set",
                "registry",
                "https://registry.npmjs.org",
                "--global",
                "--location",
                "project",
            ])
            .unwrap();
            let resolution = resolve(&yarn(version), args);
            assert_eq!(
                resolution.outcome,
                resolve(&yarn(version), set_config(Some("global"))).outcome
            );
            assert!(resolution.diagnostics.is_empty());
        }
    }

    #[test]
    fn test_bun_config_fallback_keeps_bun_program() {
        let Resolution { outcome, diagnostics } = resolve(&bun("1.3.11"), set_config(None));
        let command = expect_run(outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
        assert_eq!(
            diagnostics[0].message,
            "bun uses bunfig.toml for configuration, not a config command. Falling back to npm config."
        );
    }
}
