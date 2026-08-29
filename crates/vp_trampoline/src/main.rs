//! Minimal Windows trampoline for Vite+ shims.
//!
//! Vite+ copies and renames this binary for each shim tool.
//! Examples include `node.exe` and `npm.exe`.
//! The trampoline reads the tool name from its file name.
//! It reads the install roots from the adjacent `<name>.shim` sidecar.
//! It sets the dispatch environment for that tool.
//! It starts the active `vp.exe`.
//!
//! The trampoline ignores Ctrl+C because the child process handles it. This
//! prevents the termination prompt that `.cmd` wrappers produce.
//!
//! On Windows, `#![no_main]` and raw Win32 calls omit the CRT startup.
//! They also omit the `std::process::Command` implementation.
//! The standalone build recompiles `std` for size.
//! It uses immediate-abort panics.
//! Failure messages include the operation, path, and Windows error code.
//! See `rfcs/trampoline-exe-for-shims.md`.
//!
//! The non-Windows implementation exists for portable tests.
//! Vite+ does not ship this binary for Unix shims because they are symlinks.
//!
//! See: <https://github.com/voidzero-dev/vite-plus/issues/835>

#![cfg_attr(windows, no_main)]
#![cfg_attr(windows, windows_subsystem = "console")]

#[cfg_attr(not(windows), allow(dead_code))]
mod cmdline;
#[cfg(windows)]
mod win;

/// The linker uses this symbol as the console entry point.
/// Thus, the build does not need an `/ENTRY:` flag.
/// The `std` runtime does not initialize. See win.rs.
#[cfg(windows)]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn mainCRTStartup() -> ! {
    win::run()
}

#[cfg(not(windows))]
fn main() {
    portable::run();
}

#[cfg(any(not(windows), test))]
#[cfg_attr(windows, allow(dead_code))]
mod portable {
    use std::{
        env,
        path::{Path, PathBuf},
        process::{self, Command, ExitStatus},
    };

    use crate::cmdline::{self, ShimLayout as ParsedShimLayout};

    enum ShimLayout {
        SingleRoot,
        Split { cache: PathBuf },
    }

    struct ShimPointer {
        data: PathBuf,
        layout: ShimLayout,
    }

    struct VpLocation {
        exe: PathBuf,
        pointer: ShimPointer,
    }

    /// Return a Unix signal exit code with the shell's `128 + signal` convention.
    fn exit_code_from_status(status: ExitStatus) -> i32 {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return 128 + signal;
            }
        }
        status.code().unwrap_or(1)
    }

    /// Locate `vp.exe` from `<BIN>/<name>.shim`.
    fn resolve_vp_exe(exe_path: &Path) -> Option<VpLocation> {
        let pointer = read_shim_pointer(exe_path)?;
        let exe = pointer.data.join("current").join("bin").join("vp.exe");
        exe.exists().then_some(VpLocation { exe, pointer })
    }

    fn read_shim_pointer(exe_path: &Path) -> Option<ShimPointer> {
        let bytes = std::fs::read(exe_path.with_extension("shim")).ok()?;
        let parsed = cmdline::parse_shim_pointer(&bytes)?;
        let layout = match parsed.layout {
            ParsedShimLayout::SingleRoot => ShimLayout::SingleRoot,
            ParsedShimLayout::Split { cache } => ShimLayout::Split { cache: PathBuf::from(cache) },
        };
        Some(ShimPointer { data: PathBuf::from(parsed.data), layout })
    }

    pub fn run() {
        // 1. Determine the tool name from our own executable filename.
        let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
        let tool_name =
            exe_path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| process::exit(1));

        // 2. Locate vp.exe via `<name>.shim` (written next to every trampoline).
        let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
        let Some(location) = resolve_vp_exe(&exe_path) else {
            use std::io::Write;
            let stderr = std::io::stderr();
            let mut handle = stderr.lock();
            let _ = handle.write_all(b"vite-plus: could not locate vp.exe through .shim\n");
            process::exit(1);
        };

        // 3. Spawn vp.exe with the directory layout pinned by the sidecar.
        let mut cmd = Command::new(&location.exe);
        cmd.args(env::args_os().skip(1));
        match &location.pointer.layout {
            ShimLayout::SingleRoot => {
                cmd.env("VP_HOME", &location.pointer.data);
            }
            ShimLayout::Split { cache } => {
                cmd.env_remove("VP_HOME");
                cmd.env("VP_DATA_DIR", &location.pointer.data);
                cmd.env("VP_BIN_DIR", bin_dir);
                cmd.env("VP_CACHE_DIR", cache);
            }
        }

        if tool_name != "vp" {
            cmd.env("VP_SHIM_TOOL", tool_name);
            // A nested shim must resolve the version again.
            // It must not use passthrough mode.
            // This name must match vp_shared::env_vars::VP_TOOL_RECURSION.
            cmd.env_remove("VP_TOOL_RECURSION");
        }

        // 4. Execute and propagate the exit code.
        match cmd.status() {
            Ok(status) => process::exit(exit_code_from_status(status)),
            Err(_) => {
                use std::io::Write;
                let stderr = std::io::stderr();
                let mut handle = stderr.lock();
                let _ = handle.write_all(b"vite-plus: could not execute ");
                let _ = handle.write_all(location.exe.as_os_str().as_encoded_bytes());
                let _ = handle.write_all(b"\n");
                process::exit(1);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{fs, path::Path};

        use super::*;

        fn write_exe(path: &Path) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, b"").unwrap();
        }

        fn versioned_pointer(layout: &str, data: &Path, cache: &Path) -> String {
            format!(
                "{}\nlayout={layout}\ndata={}\ncache={}\n",
                cmdline::SHIM_POINTER_HEADER,
                data.display(),
                cache.display()
            )
        }

        #[test]
        #[cfg(unix)]
        fn preserves_signal_exit_code() {
            let status = Command::new("/bin/sh").arg("-c").arg("kill -ILL $$").status().unwrap();
            assert_eq!(exit_code_from_status(status), 132);
        }

        #[test]
        fn missing_pointer_does_not_probe_sibling_layout() {
            let root = env::temp_dir().join(format!("vp-trampoline-no-ptr-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("bin")).unwrap();
            write_exe(&root.join("current").join("bin").join("vp.exe"));
            write_exe(&root.join("data").join("current").join("bin").join("vp.exe"));

            assert!(resolve_vp_exe(&root.join("bin").join("vp.exe")).is_none());
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn pointer_without_payload_is_none() {
            let root = env::temp_dir().join(format!("vp-trampoline-empty-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("bin");
            let data = root.join("data-root");
            fs::create_dir_all(&bin).unwrap();
            fs::create_dir_all(&data).unwrap();
            fs::write(bin.join("vp.shim"), versioned_pointer("split", &data, &root.join("cache")))
                .unwrap();

            assert!(resolve_vp_exe(&bin.join("vp.exe")).is_none());
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn pointer_file_locates_data_root() {
            let root = env::temp_dir().join(format!("vp-trampoline-ptr-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("custom-bin");
            let data = root.join("custom-data");
            fs::create_dir_all(&bin).unwrap();
            write_exe(&data.join("current").join("bin").join("vp.exe"));
            fs::write(bin.join("vp.shim"), versioned_pointer("split", &data, &root.join("cache")))
                .unwrap();

            let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
            assert_eq!(location.exe, data.join("current").join("bin").join("vp.exe"));
            assert_eq!(location.pointer.data, data);
            assert!(matches!(location.pointer.layout, ShimLayout::Split { .. }));
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn unversioned_pointer_is_rejected() {
            let root = env::temp_dir().join(format!("vp-trampoline-unversioned-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("bin");
            let data = root.join("data");
            fs::create_dir_all(&bin).unwrap();
            write_exe(&data.join("current").join("bin").join("vp.exe"));
            fs::write(bin.join("vp.shim"), format!("{}\n", data.display())).unwrap();

            assert!(resolve_vp_exe(&bin.join("vp.exe")).is_none());
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn pointer_file_is_per_exe_name() {
            let root = env::temp_dir().join(format!("vp-trampoline-per-exe-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("bin");
            let node_data = root.join("node-data");
            let decoy_data = root.join("decoy-data");
            fs::create_dir_all(&bin).unwrap();
            write_exe(&node_data.join("current").join("bin").join("vp.exe"));
            write_exe(&decoy_data.join("current").join("bin").join("vp.exe"));
            fs::write(
                bin.join("vp.shim"),
                versioned_pointer("split", &decoy_data, &root.join("cache")),
            )
            .unwrap();
            fs::write(
                bin.join("node.shim"),
                versioned_pointer("split", &node_data, &root.join("cache")),
            )
            .unwrap();

            let location = resolve_vp_exe(&bin.join("node.exe")).unwrap();
            assert_eq!(location.exe, node_data.join("current").join("bin").join("vp.exe"));
            assert_eq!(location.pointer.data, node_data);
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn pointer_file_ignores_utf8_bom_and_crlf() {
            let root = env::temp_dir().join(format!("vp-trampoline-bom-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("bin");
            let data = root.join("data-root");
            fs::create_dir_all(&bin).unwrap();
            write_exe(&data.join("current").join("bin").join("vp.exe"));
            let mut contents = vec![0xEF, 0xBB, 0xBF];
            contents.extend_from_slice(
                versioned_pointer("split", &data, &root.join("cache"))
                    .replace('\n', "\r\n")
                    .as_bytes(),
            );
            fs::write(bin.join("vp.shim"), contents).unwrap();

            let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
            assert_eq!(location.exe, data.join("current").join("bin").join("vp.exe"));
            assert_eq!(location.pointer.data, data);
            assert!(matches!(location.pointer.layout, ShimLayout::Split { .. }));
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn explicit_split_does_not_become_single_root_when_bin_is_under_data() {
            let root = env::temp_dir().join(format!("vp-trampoline-split-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let data = root.join("data");
            let bin = data.join("bin");
            let cache = root.join("platform-cache");
            write_exe(&data.join("current").join("bin").join("vp.exe"));
            fs::create_dir_all(&bin).unwrap();
            fs::write(bin.join("vp.shim"), versioned_pointer("split", &data, &cache)).unwrap();

            let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
            assert!(matches!(
                location.pointer.layout,
                ShimLayout::Split { cache: value } if value == cache
            ));
            let _ = fs::remove_dir_all(&root);
        }

        #[test]
        fn explicit_single_root_sets_vp_home() {
            let root = env::temp_dir().join(format!("vp-trampoline-single-root-{}", process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join("bin");
            write_exe(&root.join("current").join("bin").join("vp.exe"));
            fs::create_dir_all(&bin).unwrap();
            fs::write(
                bin.join("vp.shim"),
                versioned_pointer("single-root", &root, &root.join("cache")),
            )
            .unwrap();

            let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
            assert!(matches!(location.pointer.layout, ShimLayout::SingleRoot));
            let _ = fs::remove_dir_all(&root);
        }
    }
}
