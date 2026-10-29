//! Prepare the downloaded release's dependencies without changing the active installation.

use std::{ffi::OsString, process::ExitCode};

use vp_setup::{VP_BINARY_NAME, install, upgrade};
use vp_shared::EnvConfig;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

pub(crate) async fn maybe_run() -> Result<Option<ExitCode>, Error> {
    let Some(protocol) = std::env::var_os(upgrade::PREPARE_ENV) else {
        return Ok(None);
    };
    if protocol != upgrade::PREPARE_PROTOCOL {
        return Err(Error::Other("Unsupported upgrade preparation protocol".into()));
    }
    let silent = parse_args(std::env::args_os().skip(1))?;
    vp_shared::validate_vp_dir_env().map_err(|error| Error::Other(error.to_string().into()))?;
    let binary = AbsolutePathBuf::new(std::env::current_exe()?).ok_or(Error::CliBinaryNotFound)?;
    let version_dir = staged_version_dir(&binary, &EnvConfig::get().dirs.data)?;
    // The target release owns all installation pins and rules. Initial-install
    // overrides, project configuration, and skip-deps flags do not apply here.
    install::generate_wrapper_package_json(&version_dir, env!("CARGO_PKG_VERSION")).await?;
    // The caller exports an explicit registry to both npm environment spellings.
    // Otherwise preserve the inherited registry precedence used by npm and pnpm.
    install::install_production_deps(&version_dir, None, !silent, None).await?;
    Ok(Some(ExitCode::SUCCESS))
}

fn parse_args(args: impl Iterator<Item = OsString>) -> Result<bool, Error> {
    let mut silent = false;
    for arg in args {
        if arg == "--silent" && !silent {
            silent = true;
        } else {
            return Err(Error::Other("Upgrade preparation accepts only --silent".into()));
        }
    }
    Ok(silent)
}

fn staged_version_dir(
    binary: &AbsolutePath,
    data: &AbsolutePath,
) -> Result<AbsolutePathBuf, Error> {
    let binary = dunce::canonicalize(binary)?;
    let data = dunce::canonicalize(data)?;
    let bin = binary.parent().ok_or(Error::CliBinaryNotFound)?;
    let version_dir = bin.parent().ok_or(Error::CliBinaryNotFound)?;
    if binary.file_name() != Some(std::ffi::OsStr::new(VP_BINARY_NAME))
        || bin.file_name() != Some(std::ffi::OsStr::new("bin"))
        || version_dir.parent() != Some(data.as_path())
    {
        return Err(Error::Other(
            "Upgrade preparation requires a staged binary in the installation data directory"
                .into(),
        ));
    }
    let current = data.join("current").join("bin").join(VP_BINARY_NAME);
    if same_file::is_same_file(&binary, &current).unwrap_or(false) {
        return Err(Error::Other("Cannot prepare an upgrade in the active installation".into()));
    }
    AbsolutePathBuf::new(version_dir.to_path_buf()).ok_or(Error::CliBinaryNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparation_accepts_only_its_own_options() {
        assert!(!parse_args([].into_iter()).unwrap());
        assert!(parse_args(["--silent".into()].into_iter()).unwrap());
        for args in [vec!["--help"], vec!["upgrade"], vec!["--silent", "--silent"]] {
            assert!(parse_args(args.into_iter().map(Into::into)).is_err());
        }
    }

    #[tokio::test]
    async fn preparation_requires_an_inactive_staged_binary() {
        let root = tempfile::tempdir().unwrap();
        let data = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let staged = data.join("1.4.0+force.test");
        let binary = staged.join("bin").join(VP_BINARY_NAME);
        tokio::fs::create_dir_all(binary.parent().unwrap()).await.unwrap();
        tokio::fs::write(&binary, b"target").await.unwrap();
        assert_eq!(
            staged_version_dir(&binary, &data).unwrap().as_path(),
            dunce::canonicalize(&staged).unwrap(),
        );

        install::swap_current_link(&data, "1.4.0+force.test").await.unwrap();
        assert!(staged_version_dir(&binary, &data).is_err());
        assert!(staged_version_dir(&data.join("current/bin").join(VP_BINARY_NAME), &data).is_err());

        let external = tempfile::tempdir().unwrap();
        let external = AbsolutePathBuf::new(external.path().to_path_buf()).unwrap();
        assert!(staged_version_dir(&binary, &external).is_err());
    }
}
