use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use serde_json::Value;

/// Exact fixture pins only. Floating versions keep their normal registry resolution.
static EXACT_VERSION: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)(?:\+[0-9A-Za-z.-]+)?$").unwrap()
});

#[derive(Default)]
pub struct BunPrewarm {
    versions: BTreeSet<String>,
    packages: BTreeSet<PathBuf>,
}

impl BunPrewarm {
    fn version(&mut self, spec: &str) {
        if let Some(version) = EXACT_VERSION.captures(spec).and_then(|captures| captures.get(1)) {
            self.versions.insert(version.as_str().to_owned());
        }
    }

    fn package_manager(&mut self, value: &Value) {
        if let Some(spec) = value.as_str().and_then(|value| value.strip_prefix("bun@")) {
            self.version(spec);
        }
    }

    fn dev_engine(&mut self, value: &Value) {
        if let Some(entries) = value.as_array() {
            for entry in entries {
                self.dev_engine(entry);
            }
        } else if value["name"] == "bun"
            && let Some(version) = value["version"].as_str()
        {
            self.version(version);
        }
    }

    pub fn package_json(&mut self, value: &Value) {
        self.package_manager(&value["packageManager"]);
        self.dev_engine(&value["devEngines"]["packageManager"]);
    }

    /// Read the same package ancestry a step sees, without scanning unrelated subprojects.
    pub fn workspace(&mut self, fixture: &Path, cwd: &str) -> std::io::Result<()> {
        let directory = fixture.join(cwd);
        for ancestor in directory.ancestors().take_while(|path| path.starts_with(fixture)) {
            let package = ancestor.join("package.json");
            if !self.packages.insert(package.clone()) {
                continue;
            }
            match std::fs::read_to_string(package) {
                Ok(text) => {
                    // Malformed package files are deliberate inputs in validation cases.
                    if let Ok(value) = serde_json::from_str(&text) {
                        self.package_json(&value);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn env<'a>(&mut self, entries: impl IntoIterator<Item = (&'a str, &'a str)>) {
        for (key, value) in entries {
            match key {
                "VP_BUN_VERSION" => self.version(value),
                "VP_PACKAGE_MANAGER" => self.package_manager(&Value::String(value.to_owned())),
                _ => {}
            }
        }
    }

    /// Inspect declarative setup, not comments, assertions, or arbitrary script source.
    pub fn step(&mut self, argv: &[String]) {
        let args: Vec<&str> = argv.iter().map(String::as_str).collect();
        match args.as_slice() {
            ["vpt", "json-edit", file, key, raw]
                if Path::new(file).file_name().is_some_and(|name| name == "package.json") =>
            {
                let value =
                    serde_json::from_str(raw).unwrap_or_else(|_| Value::String((*raw).into()));
                match *key {
                    "packageManager" => self.package_manager(&value),
                    "devEngines" => self.dev_engine(&value["packageManager"]),
                    "devEngines.packageManager" => self.dev_engine(&value),
                    _ => {}
                }
            }
            ["vpt", "write-file", file, text]
                if Path::new(file).file_name().is_some_and(|name| name == "package.json") =>
            {
                if let Ok(value) = serde_json::from_str(text) {
                    self.package_json(&value);
                }
            }
            ["vp", "env", "default" | "use" | "exec", specs @ ..] => {
                for spec in specs {
                    self.package_manager(&Value::String((*spec).into()));
                }
            }
            _ => {}
        }
    }

    pub fn urls(&self, package: &str) -> Vec<String> {
        self.versions
            .iter()
            .map(|version| {
                format!("https://registry.npmjs.org/@oven/{package}/-/{package}-{version}.tgz")
            })
            .collect()
    }
}

/// Match the native Bun archive selected by vp_pm_cli, including older x64 CPUs.
pub fn platform_package() -> String {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "aarch64") => "windows-aarch64",
        ("macos", "x86_64") => "darwin-x64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-aarch64",
        (os, arch) => panic!("unsupported Bun prewarm platform: {os}-{arch}"),
    };
    let libc = if cfg!(all(target_os = "linux", target_env = "musl")) { "-musl" } else { "" };
    #[cfg(target_arch = "x86_64")]
    let baseline = if std::arch::is_x86_feature_detected!("avx2") { "" } else { "-baseline" };
    #[cfg(not(target_arch = "x86_64"))]
    let baseline = "";
    format!("bun-{platform}{libc}{baseline}")
}
