use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkArgs {
    /// Package name or directory to link
    #[arg(value_name = "PACKAGE|DIR")]
    pub(crate) package: Option<String>,

    /// Arguments to pass to package manager
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub(crate) args: Vec<String>,
}

impl Resolve<LinkArgs> for Pnpm {
    fn resolve(&self, args: &LinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_link("pnpm", args)
    }
}

impl Resolve<LinkArgs> for Npm {
    fn resolve(&self, args: &LinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_link("npm", args)
    }
}

impl Resolve<LinkArgs> for Yarn {
    fn resolve(&self, args: &LinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_link("yarn", args)
    }
}

impl Resolve<LinkArgs> for Bun {
    fn resolve(&self, args: &LinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_link("bun", args)
    }
}

fn resolve_link(program: &str, args: &LinkArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("link");
    if let Some(package) = &args.package {
        cmd.arg(package);
    }
    cmd.extend(args.args.iter());
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_link_no_package() {
        let resolution = resolve(&pnpm("10.0.0"), LinkArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["link"]);
    }

    #[test]
    fn test_pnpm_link_package() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            LinkArgs { package: Some("react".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["link", "react"]);
    }

    #[test]
    fn test_pnpm_link_directory() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            LinkArgs { package: Some("./packages/utils".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["link", "./packages/utils"]);
    }

    #[test]
    fn test_pnpm_link_absolute_directory() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            LinkArgs {
                package: Some("/absolute/path/to/package".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["link", "/absolute/path/to/package"]);
    }

    #[test]
    fn test_yarn_link_basic() {
        let resolution = resolve(&yarn("4.0.0"), LinkArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["link"]);
    }

    #[test]
    fn test_yarn_link_package() {
        let resolution = resolve(
            &yarn("4.0.0"),
            LinkArgs { package: Some("react".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["link", "react"]);
    }

    #[test]
    fn test_yarn_classic_link_package() {
        let resolution = resolve(
            &yarn("1.22.0"),
            LinkArgs { package: Some("react".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["link", "react"]);
    }

    #[test]
    fn test_npm_link_basic() {
        let resolution = resolve(&npm("11.0.0"), LinkArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["link"]);
    }

    #[test]
    fn test_npm_link_package() {
        let resolution = resolve(
            &npm("11.0.0"),
            LinkArgs { package: Some("react".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["link", "react"]);
    }

    #[test]
    fn test_bun_link_package() {
        let resolution = resolve(
            &bun("1.3.11"),
            LinkArgs { package: Some("react".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["link", "react"]);
    }

    #[test]
    fn test_link_with_pass_through_args() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            LinkArgs { package: Some("react".to_string()), args: vec!["--global".to_string()] },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["link", "react", "--global"]);
    }

    #[test]
    fn parser_splits_package_from_trailing_args() {
        let args = parse_args::<LinkArgs>(["react", "--global"]).unwrap();

        assert_eq!(args.package, Some("react".to_string()));
        assert_eq!(args.args, vec!["--global".to_string()]);
    }
}
