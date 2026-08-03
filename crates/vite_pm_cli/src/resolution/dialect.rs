use semver::Version;

pub(crate) trait PackageManagerDialect {
    const COMMAND_NAME: &'static str;

    fn version(&self) -> Option<&Version>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Npm {
    version: Option<Version>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pnpm {
    version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Yarn {
    version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bun {
    version: Version,
}

macro_rules! impl_dialect {
    ($type:ident, $command_name:expr) => {
        impl $type {
            pub(crate) fn new(version: Version) -> Self {
                Self { version }
            }
        }

        impl PackageManagerDialect for $type {
            const COMMAND_NAME: &'static str = $command_name;

            fn version(&self) -> Option<&Version> {
                Some(&self.version)
            }
        }
    };
}

impl_dialect!(Yarn, "yarn");
impl_dialect!(Pnpm, "pnpm");
impl_dialect!(Bun, "bun");

impl Npm {
    pub(crate) fn new(version: Version) -> Self {
        Self { version: Some(version) }
    }

    pub(crate) fn unknown_version() -> Self {
        Self { version: None }
    }
}

impl PackageManagerDialect for Npm {
    const COMMAND_NAME: &'static str = "npm";

    fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }
}

impl Yarn {
    pub(crate) fn is_berry(&self) -> bool {
        self.version.major >= 2
    }
}
