use semver::Version;

use crate::resolution::{Diagnostics, PackageManagerDialect};

pub(crate) trait Diagnosis: Sized {
    fn diagnose<Dialect: PackageManagerDialect>(
        self,
        dialect: &Dialect,
        diag: &mut Diagnostics,
    ) -> Self;
}

pub(crate) trait ArgActivation {
    fn is_active(&self) -> bool;
}

impl ArgActivation for bool {
    fn is_active(&self) -> bool {
        *self
    }
}

impl<T> ArgActivation for Option<T> {
    fn is_active(&self) -> bool {
        self.is_some()
    }
}

impl<T> ArgActivation for Vec<T> {
    fn is_active(&self) -> bool {
        !self.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "#[pm_args] supports every comparison operator")]
pub(crate) enum VersionOperator {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
}

impl VersionOperator {
    pub(crate) fn matches(self, current: &Version, expected: &Version) -> bool {
        match self {
            Self::Less => current < expected,
            Self::LessEqual => current <= expected,
            Self::Greater => current > expected,
            Self::GreaterEqual => current >= expected,
            Self::Equal => current == expected,
        }
    }
}

impl std::fmt::Display for VersionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Less => write!(f, "<"),
            Self::LessEqual => write!(f, "<="),
            Self::Greater => write!(f, ">"),
            Self::GreaterEqual => write!(f, ">="),
            Self::Equal => write!(f, "="),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionRule {
    operator: VersionOperator,
    original: String,
    normalized: Version,
}

impl VersionRule {
    pub(crate) fn operator(&self) -> VersionOperator {
        self.operator
    }

    pub(crate) fn original(&self) -> &str {
        &self.original
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PmSupportRule {
    name: &'static str,
    version: Option<VersionRule>,
}

impl PmSupportRule {
    pub(crate) fn manager(name: &'static str) -> Self {
        Self { name, version: None }
    }

    pub(crate) fn version(
        name: &'static str,
        operator: VersionOperator,
        original: &str,
        normalized: Version,
    ) -> Self {
        Self {
            name,
            version: Some(VersionRule { operator, original: original.to_string(), normalized }),
        }
    }

    pub(crate) fn first_matching<'a, Dialect: PackageManagerDialect>(
        rules: &'a [Self],
        dialect: &Dialect,
    ) -> Option<&'a Self> {
        rules.iter().find(|rule| rule.matches(dialect))
    }

    pub(crate) fn manager_name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn version_rule(&self) -> Option<&VersionRule> {
        self.version.as_ref()
    }

    fn matches<Dialect: PackageManagerDialect>(&self, dialect: &Dialect) -> bool {
        if self.name != Dialect::COMMAND_NAME {
            return false;
        }
        self.version.as_ref().is_none_or(|version| {
            dialect
                .version()
                .is_some_and(|current| version.operator.matches(current, &version.normalized))
        })
    }
}

#[cfg(test)]
mod tests {
    use vite_pm_cli_macros::pm_args;

    use super::*;
    use crate::resolution::{Diagnosis, Npm, test_utils::bun};

    #[test]
    fn active_detection_filters_bool_option_vec_and_option_vec() {
        #[pm_args]
        #[derive(clap::Args, Clone, Debug, Default)]
        struct ShapeArgs {
            #[arg(long, not_supported(bun))]
            bool_field: bool,

            #[arg(long, not_supported(bun))]
            option_field: Option<String>,

            #[arg(long, value_name = "VALUE", not_supported(bun))]
            vec_field: Vec<String>,

            #[arg(long, value_name = "VALUE", not_supported(bun))]
            option_vec_field: Option<Vec<String>>,
        }

        let resolver = bun("1.3.0");
        let shape = ShapeArgs {
            bool_field: true,
            option_field: Some("value".to_string()),
            vec_field: vec!["one".to_string()],
            option_vec_field: Some(vec!["two".to_string()]),
        };
        let mut diagnostics = Diagnostics::default();
        let shape = shape.diagnose(&resolver, &mut diagnostics);
        assert!(!shape.bool_field);
        assert_eq!(shape.option_field, None);
        assert!(shape.vec_field.is_empty());
        assert_eq!(shape.option_vec_field, None);
        assert_eq!(diagnostics.len(), 4);
    }

    #[test]
    fn enum_diagnosis_filters_inline_variant_fields() {
        #[pm_args]
        #[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
        enum ShapeArgs {
            List {
                #[arg(long, not_supported(bun))]
                json: bool,
            },
            Ping,
        }

        let mut diagnostics = Diagnostics::default();
        let shape = ShapeArgs::List { json: true }.diagnose(&bun("1.3.0"), &mut diagnostics);

        assert_eq!(shape, ShapeArgs::List { json: false });
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn enum_diagnosis_binding_names_are_hygienic() {
        #[pm_args]
        #[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
        enum ShadowingArgs {
            Check {
                #[arg(long, not_supported(bun))]
                diag: bool,
                #[arg(long, not_supported(bun))]
                dialect: bool,
                #[arg(long, not_supported(bun))]
                rules: bool,
            },
        }

        let mut diagnostics = Diagnostics::default();
        let args = ShadowingArgs::Check { diag: true, dialect: true, rules: true }
            .diagnose(&bun("1.3.0"), &mut diagnostics);

        assert_eq!(args, ShadowingArgs::Check { diag: false, dialect: false, rules: false });
        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn diagnosis_preserves_type_generics_and_where_clauses() {
        #[pm_args]
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct GenericStruct<T>
        where
            T: Clone,
        {
            value: T,
        }

        #[pm_args]
        #[derive(Clone, Debug, PartialEq, Eq)]
        enum GenericEnum<T>
        where
            T: Clone,
        {
            Value { value: T },
        }

        let mut diagnostics = Diagnostics::default();
        let dialect = bun("1.3.0");
        let args =
            GenericStruct { value: "value".to_string() }.diagnose(&dialect, &mut diagnostics);
        let command =
            GenericEnum::Value { value: "value".to_string() }.diagnose(&dialect, &mut diagnostics);

        assert_eq!(args, GenericStruct { value: "value".to_string() });
        assert_eq!(command, GenericEnum::Value { value: "value".to_string() });
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnosis_preserves_conditional_fields_and_variants() {
        #[pm_args]
        #[derive(clap::Args, Clone, Debug, PartialEq, Eq)]
        struct ConditionalStruct {
            #[cfg(any())]
            #[arg(long, not_supported(bun))]
            hidden: bool,

            #[arg(long, not_supported(bun))]
            visible: bool,
        }

        #[pm_args]
        #[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
        enum ConditionalEnum {
            #[cfg_attr(all(), cfg(any()))]
            Hidden {
                #[arg(long, not_supported(bun))]
                value: bool,
            },
            Visible {
                #[cfg(any())]
                #[arg(long, not_supported(bun))]
                hidden: bool,

                #[arg(long, not_supported(bun))]
                visible: bool,
            },
        }

        let mut diagnostics = Diagnostics::default();
        let dialect = bun("1.3.0");
        let args = ConditionalStruct { visible: true }.diagnose(&dialect, &mut diagnostics);
        let command =
            ConditionalEnum::Visible { visible: true }.diagnose(&dialect, &mut diagnostics);

        assert_eq!(args, ConditionalStruct { visible: false });
        assert_eq!(command, ConditionalEnum::Visible { visible: false });
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn unknown_version_skips_version_qualified_rules() {
        #[pm_args]
        #[derive(clap::Args, Clone, Debug, PartialEq, Eq)]
        struct ShapeArgs {
            #[arg(long, not_supported(npm < "99"))]
            future: bool,
        }

        let mut diagnostics = Diagnostics::default();
        let shape = ShapeArgs { future: true }.diagnose(&Npm::unknown_version(), &mut diagnostics);

        assert!(shape.future);
        assert!(diagnostics.is_empty());
    }
}
