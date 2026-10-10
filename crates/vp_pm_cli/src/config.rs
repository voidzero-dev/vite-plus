use std::{collections::HashMap, env, ffi::OsString, fs, path::PathBuf};

use cow_utils::CowUtils;
use reqwest::{RequestBuilder, Url};
use vp_shared::EnvConfig;
use vt_path::AbsolutePath;
use vt_workspace::find_workspace_root;

const DEFAULT_NPM_REGISTRY: &str = "https://registry.npmjs.org";

/// npm configuration used while bootstrapping a package manager.
/// Authentication values stay private and are only applied to matching URLs.
#[derive(Clone)]
pub(crate) struct NpmConfig {
    pub(crate) values: HashMap<String, String>,
}

enum NpmAuth<'a> {
    Token(&'a str),
    Encoded(&'a str),
    UsernamePassword { username: &'a str, password: Vec<u8> },
}

impl NpmConfig {
    pub(crate) fn load() -> Self {
        vt_path::current_dir()
            .ok()
            .map_or_else(|| Self::load_for_project(None), |cwd| Self::load_for_cwd(&cwd))
    }

    pub(crate) fn load_for_cwd(cwd: &AbsolutePath) -> Self {
        find_workspace_root(cwd).map_or_else(
            |_| Self::load_for_project_root(cwd),
            |(root, _)| Self::load_for_project_root(&root.path),
        )
    }

    pub(crate) fn load_for_project_root(project_root: &AbsolutePath) -> Self {
        Self::load_for_project(Some(project_root.as_path().to_path_buf()))
    }

    fn load_for_project(project_root: Option<PathBuf>) -> Self {
        let mut values = HashMap::new();

        // A default global npmrc cannot be located reliably before npm exists.
        // Honor an explicitly configured one, then layer user and project config.
        if let Some(path) = env_value("globalconfig") {
            load_npmrc(config_path(&path), &mut values);
        }
        let user_config = env_value("userconfig")
            .map(|path| config_path(&path))
            .unwrap_or_else(|| EnvConfig::get().user_home.join(".npmrc").into_path_buf());
        load_npmrc(user_config, &mut values);
        if let Some(root) = project_root {
            load_npmrc(root.join(".npmrc"), &mut values);
        }

        // npm_config_* is the highest-precedence npm config source available to vp.
        for (key, value) in npm_config_env() {
            let raw_key = &key["npm_config_".len()..];
            if value.is_empty() {
                continue;
            }
            // npm preserves registry-scoped ("nerf-darted") keys verbatim.
            let key = if raw_key.starts_with("//") {
                normalize_key(&expand_value(raw_key))
            } else {
                raw_key.cow_replace('_', "-").cow_to_ascii_lowercase().into_owned()
            };
            values.insert(key, expand_value(&value));
        }
        // Keep the existing EnvConfig precedence when both spellings are set.
        if let Some(registry) = env_value("registry") {
            values.insert("registry".to_string(), expand_value(&registry));
        }
        Self { values }
    }

    pub(crate) fn registry_for_package(&self, package: &str) -> String {
        let scoped = package
            .strip_prefix('@')
            .and_then(|rest| rest.split_once('/'))
            .and_then(|(scope, _)| self.values.get(vt_str::format!("@{scope}:registry").as_str()))
            .filter(|value| !value.is_empty());
        scoped
            .or_else(|| self.values.get("registry").filter(|value| !value.is_empty()))
            .map_or_else(
                || DEFAULT_NPM_REGISTRY.to_string(),
                |value| value.trim_end_matches('/').to_string(),
            )
    }

    pub(crate) fn package_tgz_url(&self, name: &str, version: &str) -> vt_str::Str {
        let registry = self.registry_for_package(name);
        let filename = name.split('/').next_back().unwrap_or(name);
        vt_str::format!("{registry}/{name}/-/{filename}-{version}.tgz")
    }

    pub(crate) fn package_version_url(&self, name: &str, version_or_tag: &str) -> vt_str::Str {
        let registry = self.registry_for_package(name);
        vt_str::format!("{registry}/{name}/{version_or_tag}")
    }

    pub(crate) fn package_metadata_url(&self, name: &str) -> vt_str::Str {
        let registry = self.registry_for_package(name);
        vt_str::format!("{registry}/{name}")
    }

    pub(crate) fn apply_auth(&self, request: RequestBuilder, url: &str) -> RequestBuilder {
        match self.auth_for_url(url) {
            Some(NpmAuth::Token(token)) => request.bearer_auth(token),
            Some(NpmAuth::Encoded(auth)) => request
                .header(reqwest::header::AUTHORIZATION, vt_str::format!("Basic {auth}").as_str()),
            Some(NpmAuth::UsernamePassword { username, password }) => {
                request.basic_auth(username, Some(String::from_utf8_lossy(&password).as_ref()))
            }
            None => request,
        }
    }

    /// A same-origin redirect may reach a more specific authenticated path.
    /// Such responses must not use the registry-wide latest-version cache.
    pub(crate) fn has_auth_for_origin(&self, url: &str) -> bool {
        let Ok(url) = Url::parse(url) else { return false };
        let Some(host) = url.host_str() else { return false };
        let authority = url
            .port()
            .map_or_else(|| host.to_string(), |port| vt_str::format!("{host}:{port}").to_string());
        let prefix = vt_str::format!("//{}", authority.cow_to_ascii_lowercase());
        self.values.iter().any(|(key, value)| {
            key.strip_prefix(prefix.as_str()).is_some_and(|rest| {
                (rest.starts_with('/') || rest.starts_with(':'))
                    && [":_authtoken", ":_auth", ":username", ":_password"]
                        .iter()
                        .any(|suffix| rest.ends_with(suffix))
                    && !value.is_empty()
            })
        })
    }

    fn auth_for_url(&self, url: &str) -> Option<NpmAuth<'_>> {
        let Ok(url) = Url::parse(url) else { return None };
        let host = url.host_str()?;
        let authority = url
            .port()
            .map_or_else(|| host.to_string(), |port| vt_str::format!("{host}:{port}").to_string());
        // Walk the original pathname like npm-registry-fetch. Removing empty
        // segments or adding a trailing slash would broaden the credential scope.
        let mut prefix =
            vt_str::format!("//{}{}", authority.cow_to_ascii_lowercase(), url.path()).to_string();
        while prefix.len() > 2 {
            if let Some(token) = self.values.get(vt_str::format!("{prefix}:_authtoken").as_str())
                && !token.is_empty()
            {
                return Some(NpmAuth::Token(token));
            }
            if let Some(auth) = self.values.get(vt_str::format!("{prefix}:_auth").as_str())
                && !auth.is_empty()
            {
                return Some(NpmAuth::Encoded(auth));
            }
            let username = self.values.get(vt_str::format!("{prefix}:username").as_str());
            let password = self.values.get(vt_str::format!("{prefix}:_password").as_str());
            if let (Some(username), Some(password)) = (username, password)
                && !username.is_empty()
                && !password.is_empty()
                && let Ok(decoded) = base64_simd::STANDARD
                    .decode_to_vec(password)
                    .or_else(|_| base64_simd::STANDARD_NO_PAD.decode_to_vec(password))
            {
                return Some(NpmAuth::UsernamePassword { username, password: decoded });
            }
            // Remove either the final slash or the final non-slash segment.
            // This checks both directory and non-directory keys without changing
            // the request's path or collapsing repeated slashes.
            if prefix.ends_with('/') {
                prefix.pop();
            } else if let Some(slash) = prefix.rfind('/') {
                prefix.truncate(slash + 1);
            }
        }
        None
    }
}

fn env_value(name: &str) -> Option<String> {
    let lower = vt_str::format!("npm_config_{name}");
    let upper = vt_str::format!("NPM_CONFIG_{}", name.cow_to_ascii_uppercase());
    [lower.as_str(), upper.as_str()]
        .into_iter()
        .find_map(|key| env::var(key).ok().filter(|value| !value.is_empty()))
        .or_else(|| {
            npm_config_env().find_map(|(key, value)| {
                key["npm_config_".len()..]
                    .eq_ignore_ascii_case(name)
                    .then_some(value)
                    .filter(|value| !value.is_empty())
            })
        })
}

fn npm_config_env() -> impl Iterator<Item = (String, String)> {
    npm_config_env_from(env::vars_os())
}

fn npm_config_env_from(
    vars: impl Iterator<Item = (OsString, OsString)>,
) -> impl Iterator<Item = (String, String)> {
    vars.filter_map(|(key, value)| {
        let key = key.into_string().ok()?;
        let value = value.into_string().ok()?;
        key.get(.."npm_config_".len())?.eq_ignore_ascii_case("npm_config_").then_some((key, value))
    })
}

fn normalize_key(key: &str) -> String {
    let key = key.trim();
    let Some((registry, setting)) = key.rsplit_once(':').filter(|_| key.starts_with("//")) else {
        return key.cow_to_ascii_lowercase().into_owned();
    };
    let authority_end = registry[2..].find('/').map_or(registry.len(), |index| index + 2);
    vt_str::format!(
        "{}{}:{}",
        registry[..authority_end].cow_to_ascii_lowercase(),
        &registry[authority_end..],
        setting.cow_to_ascii_lowercase()
    )
    .to_string()
}

fn config_path(raw: &str) -> PathBuf {
    let path = expand_value(raw);
    let relative = path
        .strip_prefix("~/")
        .or_else(|| if cfg!(windows) { path.strip_prefix("~\\") } else { None });
    relative.map_or_else(
        || PathBuf::from(&path),
        |relative| EnvConfig::get().user_home.join(relative).into_path_buf(),
    )
}

// INI quoting is handled by parse_npmrc_value; expansion must preserve literal quotes.
fn expand_value(raw: &str) -> String {
    let value = raw.trim();
    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let Some(end) = rest[start + 2..].find('}') else {
            break;
        };
        let expression = &rest[start + 2..start + 2 + end];
        let (name, empty_if_missing) =
            expression.strip_suffix('?').map_or((expression, false), |name| (name, true));
        if name.is_empty() || name.contains(['$', '{', '?']) {
            expanded.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }
        let slashes = rest[..start].bytes().rev().take_while(|byte| *byte == b'\\').count();
        expanded.push_str(&rest[..start - slashes]);
        for _ in 0..slashes / 2 {
            expanded.push('\\');
        }
        if slashes % 2 == 1 {
            expanded.push_str(&rest[start..start + 3 + end]);
        } else {
            match env::var(name) {
                Ok(value) => expanded.push_str(&value),
                Err(_) if !empty_if_missing => expanded.push_str(&rest[start..start + 3 + end]),
                Err(_) => {}
            }
        }
        rest = &rest[start + 3 + end..];
    }
    expanded.push_str(rest);
    expanded
}

fn load_npmrc(path: PathBuf, values: &mut HashMap<String, String>) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    for raw_line in contents.trim_start_matches('\u{feff}').lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        // Expand before normalizing so case-sensitive environment names survive.
        let key = normalize_key(&expand_value(&parse_npmrc_value(key)));
        if !key.is_empty() {
            values.insert(key, expand_value(&parse_npmrc_value(value)));
        }
    }
}

fn parse_npmrc_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return serde_json::from_str(value)
            .unwrap_or_else(|_| value[1..value.len() - 1].to_string());
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return value[1..value.len() - 1].to_string();
    }

    let mut parsed = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            if !matches!(character, '\\' | '#' | ';') {
                parsed.push('\\');
            }
            parsed.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '#' | ';') {
            break;
        }
        parsed.push(character);
    }
    if escaped {
        parsed.push('\\');
    }
    parsed.trim_end().to_string()
}

/// Get the configured NPM registry URL.
#[must_use]
pub fn npm_registry() -> String {
    EnvConfig::get().npm_registry.clone()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vp_shared::env_vars;

    use super::*;

    fn project_with_npmrc(contents: &str) -> TempDir {
        let project = TempDir::new().unwrap();
        fs::write(project.path().join("package.json"), "{}").unwrap();
        fs::write(project.path().join(".npmrc"), contents).unwrap();
        project
    }

    fn http_client() -> reqwest::Client {
        vp_shared::ensure_tls_provider();
        reqwest::Client::new()
    }

    // Keep developer registry overrides and npmrc files out of fixture-based tests.
    fn with_isolated_npm_config<R>(f: impl FnOnce() -> R) -> R {
        let directory = TempDir::new().unwrap();
        let empty_config = directory.path().join("empty.npmrc");
        fs::write(&empty_config, "").unwrap();
        EnvConfig::with_vars(
            [
                (env_vars::NPM_CONFIG_REGISTRY, None),
                (env_vars::NPM_CONFIG_REGISTRY_UPPER, None),
                ("npm_config_userconfig", Some(empty_config.as_os_str())),
                ("NPM_CONFIG_USERCONFIG", Some(empty_config.as_os_str())),
                ("npm_config_globalconfig", Some(empty_config.as_os_str())),
                ("NPM_CONFIG_GLOBALCONFIG", Some(empty_config.as_os_str())),
            ],
            |_| f(),
        )
    }

    #[test]
    fn expands_user_and_global_config_paths() {
        with_isolated_npm_config(|| {
            let home = TempDir::new().unwrap();
            fs::write(home.path().join("user.npmrc"), "//registry.example/:_authToken=USER\n")
                .unwrap();
            fs::write(home.path().join("global.npmrc"), "registry=https://registry.example/\n")
                .unwrap();
            let paths = [
                ("~/user.npmrc", "${TEST_CONFIG_HOME}/global.npmrc"),
                ("${TEST_CONFIG_HOME}/user.npmrc", "~/global.npmrc"),
                #[cfg(windows)]
                ("~\\user.npmrc", "~\\global.npmrc"),
            ];
            for (user_config, global_config) in paths {
                EnvConfig::with_vars(
                    [
                        ("HOME", home.path().to_str().unwrap()),
                        ("USERPROFILE", home.path().to_str().unwrap()),
                        ("TEST_CONFIG_HOME", home.path().to_str().unwrap()),
                        ("npm_config_userconfig", user_config),
                        ("NPM_CONFIG_USERCONFIG", user_config),
                        ("npm_config_globalconfig", global_config),
                        ("NPM_CONFIG_GLOBALCONFIG", global_config),
                    ],
                    |_| {
                        let config = NpmConfig::load_for_project(None);
                        assert_eq!(config.registry_for_package("pnpm"), "https://registry.example");
                        let url = config.package_version_url("pnpm", "latest");
                        let request = config
                            .apply_auth(http_client().get(url.as_str()), &url)
                            .build()
                            .unwrap();
                        assert_eq!(
                            request.headers()[reqwest::header::AUTHORIZATION],
                            "Bearer USER"
                        );
                    },
                );
            }
        });
    }

    #[test]
    fn reads_bom_prefixed_registry_and_credentials() {
        with_isolated_npm_config(|| {
            for contents in [
                "\u{feff}registry=https://registry.example/\n//registry.example/:_authToken=TOKEN\n",
                "\u{feff}//registry.example/:_authToken=TOKEN\nregistry=https://registry.example/\n",
            ] {
                let project = project_with_npmrc(contents);
                EnvConfig::with_vars(std::iter::empty::<(&str, &str)>(), |_| {
                    let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
                    assert_eq!(config.registry_for_package("pnpm"), "https://registry.example");
                    let url = config.package_version_url("pnpm", "latest");
                    let request =
                        config.apply_auth(http_client().get(url.as_str()), &url).build().unwrap();
                    assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer TOKEN");
                });
            }
        });
    }

    #[test]
    fn expands_environment_credential_keys_and_values() {
        with_isolated_npm_config(|| {
            for (key, value) in [
                ("npm_config_//${TEST_AUTH_HOST}/Team/:_authToken", "TOKEN"),
                ("npm_config_//registry.example/Team/:_authToken", "${TEST_AUTH_TOKEN}"),
                ("npm_config_//${TEST_AUTH_HOST}/Team/:_authToken", "${TEST_AUTH_TOKEN}"),
            ] {
                EnvConfig::with_vars(
                    [
                        ("TEST_AUTH_HOST", "Registry.Example"),
                        ("TEST_AUTH_TOKEN", "TOKEN"),
                        (key, value),
                    ],
                    |_| {
                        let config = NpmConfig::load_for_project(None);
                        let url = "https://registry.example/Team/pkg";
                        let request =
                            config.apply_auth(http_client().get(url), url).build().unwrap();
                        assert_eq!(
                            request.headers()[reqwest::header::AUTHORIZATION],
                            "Bearer TOKEN",
                            "{key}={value}"
                        );
                        let other = "https://registry.example/team/pkg";
                        assert!(
                            !config
                                .apply_auth(http_client().get(other), other)
                                .build()
                                .unwrap()
                                .headers()
                                .contains_key(reqwest::header::AUTHORIZATION)
                        );
                    },
                );
            }
        });
    }

    #[test]
    fn preserves_escaped_variables_and_expands_even_backslashes() {
        with_isolated_npm_config(|| {
            EnvConfig::with_vars(
                [
                    ("TEST_EXPANSION", Some("TOKEN")),
                    ("TEST_MISSING", None),
                    // These process environment names must not make invalid npm expressions expand.
                    ("TEST?EXPANSION", Some("TOKEN")),
                    ("TEST{EXPANSION", Some("TOKEN")),
                    ("$TEST_EXPANSION", Some("TOKEN")),
                ],
                |_| {
                    for (input, expected) in [
                        (r"${TEST_EXPANSION}", "TOKEN"),
                        (r"\${TEST_EXPANSION}", "${TEST_EXPANSION}"),
                        (r"\\${TEST_EXPANSION}", r"\TOKEN"),
                        (r"\\\${TEST_EXPANSION}", r"\${TEST_EXPANSION}"),
                        (r"${TEST_MISSING}", "${TEST_MISSING}"),
                        (r"${TEST_MISSING?}", ""),
                        (r"\${TEST_MISSING?}", "${TEST_MISSING?}"),
                        (r"${TEST_EXPANSION", "${TEST_EXPANSION"),
                        (r"${}", "${}"),
                        (r"${?}", "${?}"),
                        (r"${TEST?EXPANSION}", "${TEST?EXPANSION}"),
                        (r"${TEST{EXPANSION}", "${TEST{EXPANSION}"),
                        (r"${$TEST_EXPANSION}", "${$TEST_EXPANSION}"),
                    ] {
                        assert_eq!(expand_value(input), expected, "{input}");
                    }
                    let project = project_with_npmrc(
                        "//registry.example/:_authToken=literal\\${TEST_EXPANSION}\n",
                    );
                    let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
                    let url = "https://registry.example/pkg";
                    let request = config.apply_auth(http_client().get(url), url).build().unwrap();
                    assert_eq!(
                        request.headers()[reqwest::header::AUTHORIZATION],
                        "Bearer literal${TEST_EXPANSION}"
                    );
                },
            );
        });
    }

    #[test]
    fn preserves_literal_quotes_in_basic_username() {
        with_isolated_npm_config(|| {
            let project = project_with_npmrc(
                "//registry.example/:username=\"\\\"user\\\"\"\n//registry.example/:_password=cw==\n",
            );
            EnvConfig::with_vars(std::iter::empty::<(&str, &str)>(), |_| {
                let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
                let url = "https://registry.example/pkg";
                let request = config.apply_auth(http_client().get(url), url).build().unwrap();
                assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Basic InVzZXIiOnM=");
            });
        });
    }

    #[test]
    fn accepts_padded_and_unpadded_basic_passwords() {
        for password in ["cw", "cw==", "c2U", "c2U=", "c2Vj"] {
            let config = NpmConfig {
                values: HashMap::from([
                    ("//registry.example/:username".to_string(), "user".to_string()),
                    ("//registry.example/:_password".to_string(), password.to_string()),
                ]),
            };
            let url = "https://registry.example/pkg";
            let request = config.apply_auth(http_client().get(url), url).build().unwrap();
            let expected = match password {
                "cw" | "cw==" => "Basic dXNlcjpz",
                "c2U" | "c2U=" => "Basic dXNlcjpzZQ==",
                _ => "Basic dXNlcjpzZWM=",
            };
            assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], expected);
        }
        let config = NpmConfig {
            values: HashMap::from([
                ("//registry.example/:username".to_string(), "user".to_string()),
                ("//registry.example/:_password".to_string(), "%%%".to_string()),
            ]),
        };
        let url = "https://registry.example/pkg";
        assert!(
            !config
                .apply_auth(http_client().get(url), url)
                .build()
                .unwrap()
                .headers()
                .contains_key(reqwest::header::AUTHORIZATION)
        );
    }

    #[test]
    fn test_npm_registry_default() {
        EnvConfig::with_vars([(env_vars::VP_HOME, std::env::temp_dir())], |_| {
            assert_eq!(npm_registry(), "https://registry.npmjs.org");
        });
    }

    #[test]
    fn test_npm_registry_custom() {
        EnvConfig::with_vars(
            [(env_vars::NPM_CONFIG_REGISTRY, "https://registry.npmmirror.com")],
            |_| {
                assert_eq!(npm_registry(), "https://registry.npmmirror.com");
            },
        );
    }

    #[test]
    fn test_npm_tgz_url() {
        let config = NpmConfig { values: HashMap::new() };
        assert_eq!(
            config.package_tgz_url("vite", "7.1.3"),
            "https://registry.npmjs.org/vite/-/vite-7.1.3.tgz"
        );
        assert_eq!(
            config.package_tgz_url("@vitejs/release-scripts", "1.6.0"),
            "https://registry.npmjs.org/@vitejs/release-scripts/-/release-scripts-1.6.0.tgz"
        );
    }

    #[test]
    fn reads_project_registry_and_scoped_registry() {
        let project = project_with_npmrc(
            "registry=https://default.example/\n@yarnpkg:registry=https://yarn.example/\n",
        );
        EnvConfig::with_vars(std::iter::empty::<(&'static str, &'static str)>(), |_| {
            let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
            assert_eq!(config.registry_for_package(""), "https://default.example");
            assert_eq!(config.registry_for_package("@yarnpkg/cli-dist"), "https://yarn.example");
        });
    }

    #[test]
    fn loads_registry_from_caller_provided_workspace() {
        let project = project_with_npmrc("registry=https://target.example\n");
        let cwd = AbsolutePath::new(project.path()).unwrap();
        EnvConfig::with_vars(std::iter::empty::<(&str, &str)>(), |_| {
            let config = NpmConfig::load_for_cwd(cwd);
            assert_eq!(config.registry_for_package("pnpm"), "https://target.example");
        });
    }

    #[test]
    fn loads_registry_from_caller_directory_without_a_package() {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join(".npmrc"), "registry=https://target.example\n").unwrap();
        let cwd = AbsolutePath::new(directory.path()).unwrap();
        EnvConfig::with_vars(std::iter::empty::<(&str, &str)>(), |_| {
            let config = NpmConfig::load_for_cwd(cwd);
            assert_eq!(config.registry_for_package("pnpm"), "https://target.example");
        });
    }

    #[test]
    fn empty_registry_values_fall_back() {
        let config = NpmConfig {
            values: HashMap::from([
                ("@yarnpkg:registry".to_string(), String::new()),
                ("registry".to_string(), "https://default.example/".to_string()),
            ]),
        };
        assert_eq!(config.registry_for_package("@yarnpkg/cli-dist"), "https://default.example");

        let config = NpmConfig { values: HashMap::from([("registry".to_string(), String::new())]) };
        assert_eq!(config.registry_for_package("pnpm"), DEFAULT_NPM_REGISTRY);
    }

    #[test]
    fn empty_userconfig_environment_value_is_ignored() {
        EnvConfig::with_vars([("NPM_CONFIG_USERCONFIG", "")], |_| {
            assert_eq!(env_value("userconfig"), None);
        });
    }

    #[test]
    fn npm_config_environment_prefix_is_case_insensitive() {
        let values = npm_config_env_from(
            [(OsString::from("Npm_Config_Registry"), OsString::from("https://example.test"))]
                .into_iter(),
        )
        .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![("Npm_Config_Registry".to_string(), "https://example.test".to_string())]
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_environment_entries_are_skipped() {
        use std::os::unix::ffi::OsStringExt;

        let values = npm_config_env_from(
            [
                (OsString::from_vec(vec![0xff]), OsString::from("ignored")),
                (OsString::from("NPM_CONFIG_REGISTRY"), OsString::from_vec(vec![0xff])),
                (OsString::from("NPM_CONFIG_REGISTRY"), OsString::from("https://example.test")),
            ]
            .into_iter(),
        )
        .collect::<Vec<_>>();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].1, "https://example.test");
    }

    #[test]
    fn environment_registry_overrides_project() {
        let project = project_with_npmrc("registry=https://project.example\n");
        EnvConfig::with_vars([(env_vars::NPM_CONFIG_REGISTRY, "https://env.example")], |_| {
            let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
            assert_eq!(config.registry_for_package(""), "https://env.example")
        });
    }

    #[cfg(unix)]
    #[test]
    fn lowercase_registry_environment_variable_takes_precedence() {
        EnvConfig::with_vars(
            [
                (env_vars::NPM_CONFIG_REGISTRY, "https://lower.example"),
                (env_vars::NPM_CONFIG_REGISTRY_UPPER, "https://upper.example"),
            ],
            |_| {
                let config = NpmConfig::load_for_project(None);
                assert_eq!(config.registry_for_package(""), "https://lower.example");
            },
        );
    }

    #[test]
    fn expands_auth_token_and_matches_longest_url_path() {
        let project = project_with_npmrc(
            "//registry.example/:_authToken=HOST\n//registry.example/team/:_authToken=${TEST_NPM_TOKEN}\n",
        );
        vp_shared::EnvConfig::with_vars([("TEST_NPM_TOKEN", "TEAM")], |_| {
            let request = NpmConfig::load_for_project(Some(project.path().to_path_buf()))
                .apply_auth(
                    http_client().get("https://registry.example/team/pkg"),
                    "https://registry.example/team/pkg",
                )
                .build()
                .unwrap();
            assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer TEAM");
        });
    }

    #[test]
    fn expands_registry_and_credential_keys_before_normalization() {
        let project = project_with_npmrc(
            "registry=https://${TEST_REGISTRY_HOST}/Team/\n//${TEST_REGISTRY_HOST}/Team/:_authToken=${TEST_NPM_TOKEN}\n",
        );
        EnvConfig::with_vars(
            [
                ("TEST_REGISTRY_HOST", Some("Registry.Example")),
                ("TEST_NPM_TOKEN", Some("TEAM")),
                (env_vars::NPM_CONFIG_REGISTRY, None),
                (env_vars::NPM_CONFIG_REGISTRY_UPPER, None),
            ],
            |_| {
                let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
                assert_eq!(config.registry_for_package("pnpm"), "https://Registry.Example/Team");
                let url = config.package_version_url("pnpm", "latest");
                let request =
                    config.apply_auth(http_client().get(url.as_str()), &url).build().unwrap();
                assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer TEAM");
                let other_url = "https://registry.example/team/pnpm/latest";
                let other =
                    config.apply_auth(http_client().get(other_url), other_url).build().unwrap();
                assert!(!other.headers().contains_key(reqwest::header::AUTHORIZATION));
            },
        );
    }

    #[test]
    fn empty_credentials_fall_back_to_parent_auth_path() {
        let config = NpmConfig {
            values: HashMap::from([
                ("//registry.example/team/:_authtoken".to_string(), String::new()),
                ("//registry.example/:_authtoken".to_string(), "HOST".to_string()),
            ]),
        };
        let request = config
            .apply_auth(
                http_client().get("https://registry.example/team/pkg"),
                "https://registry.example/team/pkg",
            )
            .build()
            .unwrap();
        assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer HOST");
    }

    #[test]
    fn parses_inline_comments_and_escapes_in_npmrc_values() {
        let project = project_with_npmrc(
            "registry=https://registry.example/ ; mirror\n\
             //registry.example/:_authToken=SECRET # CI\n\
             quoted=\"value # retained\"\n\
             fragment=https://example.test/\\#retained\n\
             semicolon=left\\;right ; removed\n",
        );
        // Keep other tests' registry overrides from replacing the .npmrc value.
        EnvConfig::with_vars(
            [
                (env_vars::NPM_CONFIG_REGISTRY, None::<&str>),
                (env_vars::NPM_CONFIG_REGISTRY_UPPER, None::<&str>),
            ],
            |_| {
                let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
                assert_eq!(config.registry_for_package("pnpm"), "https://registry.example");
                assert_eq!(config.values["//registry.example/:_authtoken"], "SECRET");
                assert_eq!(config.values["quoted"], "value # retained");
                assert_eq!(config.values["fragment"], "https://example.test/#retained");
                assert_eq!(config.values["semicolon"], "left;right");
            },
        );
    }

    #[test]
    fn does_not_send_auth_to_another_host() {
        let config = NpmConfig {
            values: HashMap::from([(
                "//registry.example/:_authtoken".to_string(),
                "SECRET".to_string(),
            )]),
        };
        let request = config
            .apply_auth(http_client().get("https://other.example/pkg"), "https://other.example/pkg")
            .build()
            .unwrap();
        assert!(!request.headers().contains_key(reqwest::header::AUTHORIZATION));
    }

    #[test]
    fn supports_encoded_and_username_password_basic_auth() {
        let encoded = base64_simd::STANDARD.encode_to_string("user:secret");
        let config = NpmConfig {
            values: HashMap::from([
                ("//encoded.example/:_auth".to_string(), encoded.clone()),
                ("//split.example/:username".to_string(), "user".to_string()),
                (
                    "//split.example/:_password".to_string(),
                    base64_simd::STANDARD.encode_to_string("secret"),
                ),
            ]),
        };
        for host in ["encoded.example", "split.example"] {
            let url = vt_str::format!("https://{host}/pkg");
            let request =
                config.apply_auth(http_client().get(url.as_str()), url.as_str()).build().unwrap();
            assert_eq!(
                request.headers()[reqwest::header::AUTHORIZATION],
                vt_str::format!("Basic {encoded}").as_str()
            );
        }
    }

    #[test]
    fn accepts_auth_paths_with_or_without_a_trailing_slash() {
        with_isolated_npm_config(|| {
            let project = project_with_npmrc(
                "//registry.example/team:_authToken=NO_SLASH\n//registry.example/other/:_authToken=SLASH\n",
            );
            let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
            for (path, token) in [("team/pkg", "NO_SLASH"), ("other/pkg", "SLASH")] {
                let url = vt_str::format!("https://registry.example/{path}");
                let request = config
                    .apply_auth(http_client().get(url.as_str()), url.as_str())
                    .build()
                    .unwrap();
                assert_eq!(
                    request.headers()[reqwest::header::AUTHORIZATION],
                    vt_str::format!("Bearer {token}").as_str()
                );
            }
        });
    }

    #[test]
    fn registry_auth_preserves_exact_path_boundaries() {
        let client = http_client();
        for credentials in [
            vec![("_authtoken", "SECRET".to_string())],
            vec![("_auth", base64_simd::STANDARD.encode_to_string("user:secret"))],
            vec![
                ("username", "user".to_string()),
                ("_password", base64_simd::STANDARD.encode_to_string("secret")),
            ],
        ] {
            let config = NpmConfig {
                values: credentials
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            vt_str::format!("//registry.example/team/private/:{key}").to_string(),
                            value,
                        )
                    })
                    .collect(),
            };
            for (path, authenticated) in [
                ("/team/private/pkg", true),
                ("/team/private/", true),
                ("/team/private//pkg", true),
                ("/team//private/pkg", false),
                ("//team/private/pkg", false),
                ("/team/private", false),
                ("/team/private-other/pkg", false),
            ] {
                let url = vt_str::format!("https://registry.example{path}");
                let request = config.apply_auth(client.get(url.as_str()), &url).build().unwrap();
                assert_eq!(
                    request.headers().contains_key(reqwest::header::AUTHORIZATION),
                    authenticated,
                    "unexpected authentication for {path}"
                );
                assert_eq!(request.url().path(), path);
            }
        }
    }

    #[test]
    fn registry_auth_matches_credentials_with_repeated_slashes() {
        let config = NpmConfig {
            values: HashMap::from([
                ("//registry.example/team//private/:_authtoken".to_string(), "EXACT".to_string()),
                ("//registry.example/team/:_authtoken".to_string(), "PARENT".to_string()),
            ]),
        };
        let client = http_client();
        for (path, token) in [
            ("/team//private/pkg", "EXACT"),
            ("/team/private/pkg", "PARENT"),
            ("/team//private", "PARENT"),
        ] {
            let url = vt_str::format!("https://registry.example{path}");
            let request = config.apply_auth(client.get(url.as_str()), &url).build().unwrap();
            assert_eq!(
                request.headers()[reqwest::header::AUTHORIZATION],
                vt_str::format!("Bearer {token}").as_str()
            );
        }
    }

    #[test]
    fn registry_auth_paths_remain_case_sensitive() {
        with_isolated_npm_config(|| {
            let project = project_with_npmrc("//registry.example/Team/:_authToken=SECRET\n");
            let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));

            let matching = config
                .apply_auth(
                    http_client().get("https://registry.example/Team/pkg"),
                    "https://registry.example/Team/pkg",
                )
                .build()
                .unwrap();
            assert_eq!(matching.headers()[reqwest::header::AUTHORIZATION], "Bearer SECRET");

            let different_case = config
                .apply_auth(
                    http_client().get("https://registry.example/team/pkg"),
                    "https://registry.example/team/pkg",
                )
                .build()
                .unwrap();
            assert!(!different_case.headers().contains_key(reqwest::header::AUTHORIZATION));
        });
    }
}
