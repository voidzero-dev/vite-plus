use thiserror::Error;
use vt_path::AbsolutePathBuf;

use crate::env_vars;

enum PathOverride {
    Unset,
    Absolute(AbsolutePathBuf),
    Relative,
}

impl PathOverride {
    fn from_env(name: &str) -> Self {
        let Some(path) = std::env::var_os(name).filter(|value| !value.is_empty()) else {
            return Self::Unset;
        };
        AbsolutePathBuf::new(path.into()).map_or(Self::Relative, Self::Absolute)
    }

    fn absolute(&self) -> Option<&AbsolutePathBuf> {
        match self {
            Self::Absolute(path) => Some(path),
            Self::Unset | Self::Relative => None,
        }
    }

    fn is_set(&self) -> bool {
        !matches!(self, Self::Unset)
    }

    fn is_relative(&self) -> bool {
        matches!(self, Self::Relative)
    }
}

/// One snapshot of the Vite+ directory override variables.
pub(super) struct DirEnvOverrides {
    home: PathOverride,
    bin: PathOverride,
    data: PathOverride,
    cache: PathOverride,
}

impl DirEnvOverrides {
    pub(super) fn from_env() -> Self {
        Self {
            home: PathOverride::from_env(env_vars::VP_HOME),
            bin: PathOverride::from_env(env_vars::VP_BIN_DIR),
            data: PathOverride::from_env(env_vars::VP_DATA_DIR),
            cache: PathOverride::from_env(env_vars::VP_CACHE_DIR),
        }
    }

    pub(super) fn home(&self) -> Option<AbsolutePathBuf> {
        self.home.absolute().cloned()
    }

    pub(super) fn split_dirs(&self) -> Option<(AbsolutePathBuf, AbsolutePathBuf, AbsolutePathBuf)> {
        Some((
            self.bin.absolute()?.clone(),
            self.data.absolute()?.clone(),
            self.cache.absolute()?.clone(),
        ))
    }

    fn validate(&self) -> Result<(), VpDirEnvError> {
        if self.home.is_relative() {
            return Err(VpDirEnvError::RelativePath { name: env_vars::VP_HOME });
        }

        let split_dirs = [
            (env_vars::VP_BIN_DIR, &self.bin),
            (env_vars::VP_DATA_DIR, &self.data),
            (env_vars::VP_CACHE_DIR, &self.cache),
        ];
        let configured_count = split_dirs.iter().filter(|(_, value)| value.is_set()).count();
        if configured_count != 0 && configured_count != split_dirs.len() {
            return Err(VpDirEnvError::IncompleteSplitGroup);
        }
        for (name, value) in split_dirs {
            if value.is_relative() {
                return Err(VpDirEnvError::RelativePath { name });
            }
        }

        Ok(())
    }
}

/// An error in the Vite+ directory overrides.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VpDirEnvError {
    #[error(
        "Set all three variables together: VP_BIN_DIR, VP_DATA_DIR, and VP_CACHE_DIR. Otherwise, do not set any of them."
    )]
    IncompleteSplitGroup,
    #[error("Set {name} to an absolute path.")]
    RelativePath { name: &'static str },
}

/// Validate Vite+ directory overrides for an installer.
///
/// `VpDirs` ignores invalid overrides during runtime resolution. It then checks
/// the next directory source. Installers call this function before they resolve
/// or create installation roots.
pub fn validate_vp_dir_env() -> Result<(), VpDirEnvError> {
    DirEnvOverrides::from_env().validate()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn validate_with(
        home: Option<&OsStr>,
        [bin, data, cache]: [Option<&OsStr>; 3],
    ) -> Result<(), VpDirEnvError> {
        temp_env::with_vars(
            [
                (env_vars::VP_HOME, home),
                (env_vars::VP_BIN_DIR, bin),
                (env_vars::VP_DATA_DIR, data),
                (env_vars::VP_CACHE_DIR, cache),
            ],
            validate_vp_dir_env,
        )
    }

    #[test]
    fn installer_validation_accepts_valid_overrides() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let data = root.path().join("data");
        let cache = root.path().join("cache");

        assert_eq!(validate_with(None, [None, None, None]), Ok(()));
        assert_eq!(validate_with(Some(root.path().as_os_str()), [None, None, None]), Ok(()));
        assert_eq!(
            validate_with(
                None,
                [Some(bin.as_os_str()), Some(data.as_os_str()), Some(cache.as_os_str())],
            ),
            Ok(())
        );
    }

    #[test]
    fn installer_validation_rejects_incomplete_split_groups() {
        let root = tempfile::tempdir().unwrap();
        let paths = [root.path().join("bin"), root.path().join("data"), root.path().join("cache")];
        let paths = paths.each_ref().map(|path| path.as_os_str());
        let cases = [
            [Some(paths[0]), None, None],
            [None, Some(paths[1]), None],
            [None, None, Some(paths[2])],
            [Some(paths[0]), Some(paths[1]), None],
            [Some(paths[0]), None, Some(paths[2])],
            [None, Some(paths[1]), Some(paths[2])],
        ];

        for dirs in cases {
            assert_eq!(validate_with(None, dirs), Err(VpDirEnvError::IncompleteSplitGroup));
        }
    }

    #[test]
    fn installer_validation_rejects_relative_overrides() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let data = root.path().join("data");
        let cache = root.path().join("cache");
        let relative = OsStr::new("relative-dir");

        assert_eq!(
            validate_with(Some(relative), [None, None, None]),
            Err(VpDirEnvError::RelativePath { name: env_vars::VP_HOME })
        );

        let cases = [
            (
                env_vars::VP_BIN_DIR,
                [Some(relative), Some(data.as_os_str()), Some(cache.as_os_str())],
            ),
            (
                env_vars::VP_DATA_DIR,
                [Some(bin.as_os_str()), Some(relative), Some(cache.as_os_str())],
            ),
            (
                env_vars::VP_CACHE_DIR,
                [Some(bin.as_os_str()), Some(data.as_os_str()), Some(relative)],
            ),
        ];
        for (name, dirs) in cases {
            assert_eq!(validate_with(None, dirs), Err(VpDirEnvError::RelativePath { name }));
        }
    }
}
