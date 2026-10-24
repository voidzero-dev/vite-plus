//! Run registry redirection in a dedicated test process, never alongside other tests.

use std::process::Command;

use vp_shared::{EnvConfig, env_vars};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

struct RegistryOverride {
    root: RegKey,
    path: String,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOverridePredefKey(key: isize, replacement: isize) -> i32;
}

impl RegistryOverride {
    fn new() -> Self {
        let path = format!("Software\\VitePlusTests\\{}", uuid::Uuid::new_v4());
        let (root, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&path).unwrap();
        // This process runs only the ignored helper test below. Keep the root
        // handle alive and restore HKCU before removing the temporary subtree.
        assert_eq!(
            unsafe { RegOverridePredefKey(HKEY_CURRENT_USER as isize, root.raw_handle() as isize) },
            0
        );
        Self { root, path }
    }
}

impl Drop for RegistryOverride {
    fn drop(&mut self) {
        assert_eq!(unsafe { RegOverridePredefKey(HKEY_CURRENT_USER as isize, 0) }, 0);
        RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.path).unwrap();
    }
}

#[test]
fn windows_self_setup_handles_path_failures_and_optional_shells() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "self_setup::windows_tests::isolated_setup", "--nocapture"])
        .env("VP_TEST_ISOLATED_REGISTRY", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "invoked in a dedicated process by windows_self_setup_handles_path_failures_and_optional_shells"]
async fn isolated_setup() {
    assert_eq!(std::env::var("VP_TEST_ISOLATED_REGISTRY").as_deref(), Ok("1"));
    let registry = RegistryOverride::new();
    let (environment, _) = registry.root.create_subkey("Environment").unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let source_dir = temporary.path().join("source");
    std::fs::create_dir(&source_dir).unwrap();
    let source = source_dir.join("vp.exe");
    // run() only copies these payloads and creates shims; it never executes them.
    std::fs::write(&source, b"vp test payload").unwrap();
    std::fs::write(source_dir.join("vp-shim.exe"), b"test trampoline").unwrap();
    let nu_dir = temporary.path().join("old-nu");
    std::fs::create_dir(&nu_dir).unwrap();
    // The test harness rejects "-c", like Nushell versions without the queried field.
    std::fs::copy(std::env::current_exe().unwrap(), nu_dir.join("nu.exe")).unwrap();

    for (case, path, skip) in [
        ("no-powershell", "", None),
        ("broken-nushell", nu_dir.to_str().unwrap(), None),
        ("path-error", "", None),
        ("opt-out", "", Some("1")),
    ] {
        let home = temporary.path().join(case);
        let home_str = home.to_str().unwrap();
        let environment = &environment;
        let source = &source;
        Box::pin(EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, Some(home_str)),
                ("PATH", Some(path)),
                ("VP_SKIP_DEPS_INSTALL", Some("1")),
                ("VP_VERSION", Some("1.1.0")),
                ("VP_NODE_MANAGER", Some("no")),
                ("VP_PM_MANAGER", Some("no")),
                (env_vars::VP_SELF_SETUP_NO_MODIFY_PATH, skip),
                ("CI", Some("true")),
            ],
            |config| async move {
                let before = "%PNPM_HOME%;;C:\\tools";
                environment.set_value("Path", &before).unwrap();
                let marker = config.dirs.data.join("1.1.0/bin").join(vp_setup::SELF_SETUP_MARKER);
                if case == "path-error" {
                    environment.set_value("Path", &42_u32).unwrap();
                    let error = super::run(source, false).await.unwrap_err();
                    assert!(error.to_string().contains("Could not configure user PATH"), "{error}");
                    assert!(!marker.as_path().is_file(), "failed PATH setup must remain retryable");
                    assert_eq!(environment.get_value::<u32, _>("Path").unwrap(), 42);
                    // Retrying after the registry value is repaired finishes setup.
                    environment.set_value("Path", &before).unwrap();
                }
                let binary = super::run(source, false).await.unwrap();
                assert!(marker.as_path().is_file());
                let persisted: String = environment.get_value("Path").unwrap();
                if skip.is_some() {
                    assert_eq!(persisted, before);
                } else {
                    assert_eq!(
                        persisted,
                        format!(
                            "{};{before};{}",
                            config.dirs.bin.as_path().display(),
                            config.dirs.fallback_bin().as_path().display()
                        )
                    );
                }
                // An upgrade invokes the binary already in current/bin after
                // clearing its marker. It must configure PATH with the same rules.
                std::fs::remove_file(marker.as_path()).unwrap();
                environment.set_value("Path", &before).unwrap();
                super::run(binary.as_path(), false).await.unwrap();
                assert!(marker.as_path().is_file());
                assert_eq!(environment.get_value::<String, _>("Path").unwrap(), persisted);
            },
        ))
        .await;
    }
}
