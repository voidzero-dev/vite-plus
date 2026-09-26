//! Fetch the setup pnpm with Node's npm so registry authentication follows npmrc.

use std::path::Component;

use serde::Deserialize;
use tokio::process::Command;
use vp_error::Error;
use vp_shared::{EnvConfig, env_vars};
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;

use super::{
    PackageManagerType, create_shim_files, find_extracted_package_dir,
    is_package_manager_install_complete, open_lock_file, remove_dir_all_force,
};

/// Keep npm and pnpm on the same explicit registry and user config when their
/// working directory differs from the caller's. Credentials stay in npm's
/// normal configuration and inherited environment, never in command arguments.
pub fn configure_npm_command(command: &mut Command, registry: Option<&str>) -> Result<(), Error> {
    if let Some(registry) = registry {
        command.env(env_vars::NPM_CONFIG_REGISTRY, registry);
        command.env(env_vars::NPM_CONFIG_REGISTRY_UPPER, registry);
    }
    if let Some(path) = std::env::var_os("npm_config_userconfig")
        .or_else(|| std::env::var_os("NPM_CONFIG_USERCONFIG"))
        .filter(|path| !path.is_empty())
    {
        let path = vt_path::current_dir()?.join(path);
        command.env("npm_config_userconfig", path.as_path());
        command.env("NPM_CONFIG_USERCONFIG", path.as_path());
    }
    Ok(())
}

#[derive(Deserialize)]
struct PackedPackage {
    name: Str,
    version: Str,
    filename: Str,
}

fn packed_filename(output: &[u8], version: &str) -> Result<Str, Error> {
    let packages: Vec<PackedPackage> = serde_json::from_slice(output)?;
    let [package] = packages.as_slice() else {
        return Err(Error::InvalidArgument("npm pack must return one pnpm archive".into()));
    };
    let mut components = std::path::Path::new(package.filename.as_str()).components();
    if package.name != "pnpm"
        || package.version != version
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || !package.filename.ends_with(".tgz")
    {
        return Err(Error::InvalidArgument("npm pack returned an unexpected pnpm archive".into()));
    }
    Ok(package.filename.clone())
}

/// Bootstrap a pinned JavaScript pnpm package with the npm included in a managed
/// Node distribution. Reuse the normal cache and lock so later vp commands see
/// the same completed package-manager installation.
pub async fn bootstrap_pnpm(
    node: &AbsolutePath,
    version: &str,
    registry: Option<&str>,
) -> Result<AbsolutePathBuf, Error> {
    let parsed = semver::Version::parse(version)
        .map_err(|_| Error::InvalidArgument("pnpm bootstrap requires an exact version".into()))?;
    if parsed.major >= 12 {
        return Err(Error::InvalidArgument(
            "pnpm bootstrap requires a JavaScript pnpm release".into(),
        ));
    }
    let target = EnvConfig::get().dirs.data.join("package_manager/pnpm").join(version);
    let install = target.join("pnpm");
    let complete = || {
        Ok::<_, Error>(
            is_package_manager_install_complete(&install, "pnpm")?
                && install.join("bin/pnpm.cjs").as_path().is_file(),
        )
    };
    if complete()? {
        return Ok(install);
    }

    let parent = target.parent().expect("pnpm cache has a parent");
    tokio::fs::create_dir_all(parent).await?;
    let lock_path = parent.join(vt_str::format!("{version}.lock"));
    // File::lock can block; keep it off the async executor during concurrent setup.
    let _lock = tokio::task::spawn_blocking(move || {
        let file = open_lock_file(lock_path.as_path())?;
        file.lock()?;
        Ok::<_, std::io::Error>(file)
    })
    .await??;
    if complete()? {
        return Ok(install);
    }

    let temporary = tempfile::tempdir_in(parent)?;
    let work = AbsolutePathBuf::new(temporary.path().to_path_buf()).expect("absolute tempdir");
    let node_bin =
        node.parent().ok_or_else(|| Error::CannotFindBinaryPath("Node directory".into()))?;
    let npm = node_bin.join(if cfg!(windows) {
        "node_modules/npm/bin/npm-cli.js"
    } else {
        "../lib/node_modules/npm/bin/npm-cli.js"
    });
    if !npm.as_path().is_file() {
        return Err(Error::CannotFindBinaryPath(
            "npm bundled with the managed Node.js runtime".into(),
        ));
    }
    // Establish a package root even if the user chose a data directory under a workspace.
    tokio::fs::write(work.join("package.json"), br#"{"name":"vp-bootstrap","private":true}"#)
        .await?;
    let mut command = Command::new(node.as_path());
    command
        .arg(npm.as_path())
        .args([
            "pack",
            &vt_str::format!("pnpm@{version}"),
            "--ignore-scripts",
            "--json",
            "--workspaces=false",
            "--logs-max=0",
        ])
        .current_dir(&work);
    configure_npm_command(&mut command, registry)?;
    let output = command.output().await?;
    if !output.status.success() {
        // Registry error bodies can contain credentials. Do not log subprocess output.
        return Err(Error::InvalidArgument(vt_str::format!(
            "Failed to download pnpm {version} (exit code {}). Check your npm registry and authentication configuration.",
            vp_shared::exit_code_from_status(output.status)
        )));
    }
    let archive = work.join(packed_filename(&output.stdout, version)?);
    let extracted = work.join("extracted");
    let destination = extracted.clone();
    tokio::task::spawn_blocking(move || crate::request::extract_tgz(&archive, &destination))
        .await??;
    let package = find_extracted_package_dir(extracted.as_path())?;
    let staged = work.join("installation");
    tokio::fs::create_dir(&staged).await?;
    tokio::fs::rename(package, staged.join("pnpm")).await?;
    let staged_install = staged.join("pnpm");
    create_shim_files(PackageManagerType::Pnpm, staged_install.join("bin")).await?;
    if !is_package_manager_install_complete(&staged_install, "pnpm")?
        || !staged_install.join("bin/pnpm.cjs").as_path().is_file()
    {
        return Err(Error::CannotFindBinaryPath("pnpm bootstrap archive has no CLI".into()));
    }
    remove_dir_all_force(&target).await?;
    tokio::fs::rename(staged, &target).await?;
    Ok(install)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_versions_and_archive_paths() {
        let output = |name: &str, version: &str, filename: &str| {
            serde_json::to_vec(
                &serde_json::json!([{ "name": name, "version": version, "filename": filename }]),
            )
            .unwrap()
        };
        assert_eq!(
            packed_filename(&output("pnpm", "10.33.0", "pnpm-10.33.0.tgz"), "10.33.0").unwrap(),
            "pnpm-10.33.0.tgz"
        );
        for (name, version, filename) in [
            ("npm", "10.33.0", "pnpm.tgz"),
            ("pnpm", "10.32.0", "pnpm.tgz"),
            ("pnpm", "10.33.0", "../pnpm.tgz"),
            ("pnpm", "10.33.0", "/pnpm.tgz"),
            ("pnpm", "10.33.0", "dir/pnpm.tgz"),
            ("pnpm", "10.33.0", ""),
        ] {
            assert!(packed_filename(&output(name, version, filename), "10.33.0").is_err());
        }
        assert!(packed_filename(b"[]", "10.33.0").is_err());
        assert!(packed_filename(b"invalid JSON", "10.33.0").is_err());
    }

    #[test]
    fn keeps_explicit_userconfig_relative_to_the_caller() {
        EnvConfig::with_vars(
            [
                ("npm_config_userconfig", None),
                ("NPM_CONFIG_USERCONFIG", Some("config/enterprise.npmrc")),
            ],
            |_| {
                let mut command = Command::new("node");
                configure_npm_command(&mut command, Some("https://registry.example.test/"))
                    .unwrap();
                let expected = vt_path::current_dir().unwrap().join("config/enterprise.npmrc");
                for (name, value) in command.as_std().get_envs() {
                    match name.to_str().unwrap() {
                        "npm_config_userconfig" | "NPM_CONFIG_USERCONFIG" => {
                            assert_eq!(value.unwrap(), expected.as_path())
                        }
                        "npm_config_registry" | "NPM_CONFIG_REGISTRY" => {
                            assert_eq!(value.unwrap(), "https://registry.example.test/")
                        }
                        other => panic!("unexpected environment setting {other}"),
                    }
                }
                assert_eq!(command.as_std().get_args().len(), 0);
            },
        );
    }
}
