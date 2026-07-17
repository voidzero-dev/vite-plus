use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewArgs {
    /// Package name with optional version
    #[arg(required = true)]
    pub(crate) package: String,

    /// Specific field to view
    pub(crate) field: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub(crate) json: bool,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<ViewArgs> for Pnpm {
    fn resolve(&self, args: &ViewArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_simple_view("pnpm", "view", args)
    }
}

impl Resolve<ViewArgs> for Npm {
    fn resolve(&self, args: &ViewArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_simple_view("npm", "view", args)
    }
}

impl Resolve<ViewArgs> for Yarn {
    fn resolve(&self, args: &ViewArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if !self.is_berry() {
            return resolve_simple_view("yarn", "info", args);
        }

        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("npm").arg("info").arg(&args.package);
        if let Some(field) = &args.field {
            cmd.arg("--fields").arg(field);
        }
        cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ViewArgs> for Bun {
    fn resolve(&self, args: &ViewArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_simple_view("bun", "info", args)
    }
}

fn resolve_simple_view(program: &str, subcommand: &str, args: &ViewArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg(subcommand).arg(&args.package);
    if let Some(field) = &args.field {
        cmd.arg(field);
    }
    cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, pnpm, yarn},
    };

    fn view_args(package: &str) -> ViewArgs {
        ViewArgs { package: package.to_string(), ..Default::default() }
    }

    #[test]
    fn test_pnpm_view_uses_pnpm() {
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), view_args("react")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["view", "react"]);
    }

    #[test]
    fn test_npm_view() {
        let mut options = view_args("react");
        options.field = Some("version".to_string());
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["view", "react", "version"]);
    }

    #[test]
    fn test_yarn_view_uses_info() {
        let mut options = view_args("lodash");
        options.json = true;
        let CommandResolution::Run(command) = resolve(&yarn("1.22.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["info", "lodash", "--json"]);
    }

    #[test]
    fn test_yarn_berry_view_uses_yarn_npm_info() {
        let mut options = view_args("lodash");
        options.json = true;
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "info", "lodash", "--json"]);
    }

    #[test]
    fn test_yarn_berry_view_uses_fields_option_for_view_field() {
        let mut options = view_args("lodash");
        options.field = Some("dist.tarball".to_string());
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "info", "lodash", "--fields", "dist.tarball"]);
    }

    #[test]
    fn test_view_with_nested_field() {
        let mut options = view_args("react");
        options.field = Some("dist.tarball".to_string());
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["view", "react", "dist.tarball"]);
    }

    #[test]
    fn test_bun_view_uses_info() {
        let mut options = view_args("react");
        options.field = Some("version".to_string());
        options.json = true;
        let CommandResolution::Run(command) = resolve(&bun("1.3.11"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["info", "react", "version", "--json"]);
    }

    #[test]
    fn test_view_with_pass_through_args() {
        let mut options = view_args("react");
        options.pass_through_args =
            vec!["--registry".to_string(), "https://registry.npmjs.org".to_string()];
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["view", "react", "--registry", "https://registry.npmjs.org"]);
    }
}
