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

impl NpmConfig {
    pub(crate) fn load() -> Self {
        vt_path::current_dir()
            .ok()
            .map_or_else(|| Self::load_for_project(None), |cwd| Self::load_for_cwd(&cwd))
    }

    pub(crate) fn load_for_cwd(cwd: &AbsolutePath) -> Self {
        find_workspace_root(cwd).map_or_else(
            |_| Self::load_for_project(None),
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
            load_npmrc(PathBuf::from(path), &mut values);
        }
        let user_config = env_value("userconfig")
            .map(PathBuf::from)
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
                normalize_key(raw_key)
            } else {
                raw_key.cow_replace('_', "-").cow_to_ascii_lowercase().into_owned()
            };
            values.insert(key, value);
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
        let Ok(url) = Url::parse(url) else { return request };
        let Some(host) = url.host_str() else { return request };
        let authority = url
            .port()
            .map_or_else(|| host.to_string(), |port| vt_str::format!("{host}:{port}").to_string());
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|segment| !segment.is_empty())
            .collect();

        // Match npm-registry-fetch: the most specific URL path wins.
        for length in (0..=segments.len()).rev() {
            let path = if length == 0 {
                "/".to_string()
            } else {
                vt_str::format!("/{}/", segments[..length].join("/")).to_string()
            };
            let prefix = vt_str::format!("//{}{path}", authority.cow_to_ascii_lowercase());
            for prefix in [prefix.as_str(), prefix.trim_end_matches('/')] {
                if let Some(token) =
                    self.values.get(vt_str::format!("{prefix}:_authtoken").as_str())
                    && !token.is_empty()
                {
                    return request.bearer_auth(token);
                }
                if let Some(auth) = self.values.get(vt_str::format!("{prefix}:_auth").as_str())
                    && !auth.is_empty()
                {
                    return request.header(
                        reqwest::header::AUTHORIZATION,
                        vt_str::format!("Basic {auth}").as_str(),
                    );
                }
                let username = self.values.get(vt_str::format!("{prefix}:username").as_str());
                let password = self.values.get(vt_str::format!("{prefix}:_password").as_str());
                if let (Some(username), Some(password)) = (username, password)
                    && !username.is_empty()
                    && !password.is_empty()
                    && let Ok(decoded) = base64_simd::STANDARD.decode_to_vec(password)
                {
                    return request
                        .basic_auth(username, Some(String::from_utf8_lossy(&decoded).as_ref()));
                }
            }
        }
        request
    }
}

fn env_value(name: &str) -> Option<String> {
    npm_config_env().find_map(|(key, value)| {
        key["npm_config_".len()..]
            .eq_ignore_ascii_case(name)
            .then_some(value)
            .filter(|value| !value.is_empty())
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

fn expand_value(raw: &str) -> String {
    let mut value = raw.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value = &value[1..value.len() - 1];
    }

    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        expanded.push_str(&rest[..start]);
        let Some(end) = rest[start + 2..].find('}') else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let expression = &rest[start + 2..start + 2 + end];
        let (name, empty_if_missing) =
            expression.strip_suffix('?').map_or((expression, false), |name| (name, true));
        match env::var(name) {
            Ok(value) => expanded.push_str(&value),
            Err(_) if !empty_if_missing => expanded.push_str(&rest[start..start + 3 + end]),
            Err(_) => {}
        }
        rest = &rest[start + 3 + end..];
    }
    expanded.push_str(rest);
    expanded
}

fn load_npmrc(path: PathBuf, values: &mut HashMap<String, String>) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = normalize_key(key);
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

/// Get the configured default NPM registry URL.
#[must_use]
pub fn npm_registry() -> String {
    NpmConfig::load().registry_for_package("")
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
        let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
        assert_eq!(config.registry_for_package("pnpm"), "https://registry.example");
        assert_eq!(config.values["//registry.example/:_authtoken"], "SECRET");
        assert_eq!(config.values["quoted"], "value # retained");
        assert_eq!(config.values["fragment"], "https://example.test/#retained");
        assert_eq!(config.values["semicolon"], "left;right");
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
        let project = project_with_npmrc(
            "//registry.example/team:_authToken=NO_SLASH\n//registry.example/other/:_authToken=SLASH\n",
        );
        let config = NpmConfig::load_for_project(Some(project.path().to_path_buf()));
        for (path, token) in [("team/pkg", "NO_SLASH"), ("other/pkg", "SLASH")] {
            let url = vt_str::format!("https://registry.example/{path}");
            let request =
                config.apply_auth(http_client().get(url.as_str()), url.as_str()).build().unwrap();
            assert_eq!(
                request.headers()[reqwest::header::AUTHORIZATION],
                vt_str::format!("Bearer {token}").as_str()
            );
        }
    }

    #[test]
    fn registry_auth_paths_remain_case_sensitive() {
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
    }
}
