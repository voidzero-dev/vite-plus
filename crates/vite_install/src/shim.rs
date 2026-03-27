use std::path::Path;

use indoc::formatdoc;
use pathdiff::diff_paths;
use tokio::fs::write;
use vite_error::Error;

/// Write cmd/sh/pwsh shim files for native (non-Node.js) binaries.
/// Unlike `write_shims` which wraps JS files with Node.js, this creates
/// wrappers that exec the native binary directly.
pub async fn write_native_shims(
    source_file: impl AsRef<Path>,
    to_bin: impl AsRef<Path>,
) -> Result<(), Error> {
    let to_bin = to_bin.as_ref();
    let parent = to_bin
        .parent()
        .ok_or_else(|| Error::CannotFindBinaryPath("shim path has no parent directory".into()))?;
    let relative_path = diff_paths(&source_file, parent).ok_or_else(|| {
        Error::CannotFindBinaryPath("cannot compute relative path for shim".into())
    })?;
    let relative_file = relative_path
        .to_str()
        .ok_or_else(|| Error::CannotFindBinaryPath("shim path is not valid UTF-8".into()))?;

    write(to_bin, native_sh_shim(relative_file)).await?;
    write(to_bin.with_extension("cmd"), native_cmd_shim(relative_file)).await?;
    write(to_bin.with_extension("ps1"), native_pwsh_shim(relative_file)).await?;

    // set executable permission for unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(to_bin, std::fs::Permissions::from_mode(0o755)).await?;
    }

    tracing::debug!("write_native_shims: {:?} -> {:?}", to_bin, relative_file);
    Ok(())
}

/// Unix shell shim for native binaries.
pub fn native_sh_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        #!/bin/sh
        basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

        case `uname` in
            *CYGWIN*|*MINGW*|*MSYS*)
                if command -v cygpath > /dev/null 2>&1; then
                    basedir=`cygpath -w "$basedir"`
                fi
            ;;
        esac

        exec "$basedir/{relative_file}" "$@"
        "#
    }
}

/// Windows Command Prompt shim for native binaries.
pub fn native_cmd_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        @SETLOCAL
        @"%~dp0\{relative_file}" %*
        "#,
        relative_file = relative_file.replace('/', "\\")
    }
    .replace('\n', "\r\n")
}

/// `PowerShell` shim for native binaries.
pub fn native_pwsh_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        #!/usr/bin/env pwsh
        $basedir=Split-Path $MyInvocation.MyCommand.Definition -Parent

        $ret=0
        # Support pipeline input
        if ($MyInvocation.ExpectingInput) {{
            $input | & "$basedir/{relative_file}" $args
        }} else {{
            & "$basedir/{relative_file}" $args
        }}
        $ret=$LASTEXITCODE
        exit $ret
        "#
    }
}

/// Write cmd/sh/pwsh shim files.
pub async fn write_shims(
    source_file: impl AsRef<Path>,
    to_bin: impl AsRef<Path>,
) -> Result<(), Error> {
    let to_bin = to_bin.as_ref();
    // source file `/foo/bar/pnpm.js` point to bin file `/foo/bin/npm`, the relative path is `../bar/pnpm.js`.
    let relative_path = diff_paths(source_file, to_bin.parent().unwrap()).unwrap();
    let relative_file = relative_path.to_str().unwrap();

    // Referenced from pnpm/cmd-shim's TypeScript implementation:
    // https://github.com/pnpm/cmd-shim/blob/main/src/index.ts
    write(to_bin, sh_shim(relative_file)).await?;
    write(to_bin.with_extension("cmd"), cmd_shim(relative_file)).await?;
    write(to_bin.with_extension("ps1"), pwsh_shim(relative_file)).await?;

    // set executable permission for unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(to_bin, std::fs::Permissions::from_mode(0o755)).await?;
    }

    tracing::debug!("write_shims: {:?} -> {:?}", to_bin, relative_file);
    Ok(())
}

/// Unix shell shim.
pub fn sh_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        #!/bin/sh
        basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

        case `uname` in
            *CYGWIN*|*MINGW*|*MSYS*)
                if command -v cygpath > /dev/null 2>&1; then
                    basedir=`cygpath -w "$basedir"`
                fi
            ;;
        esac

        if [ -x "$basedir/node" ]; then
            exec "$basedir/node" "$basedir/{relative_file}" "$@"
        else
            exec node "$basedir/{relative_file}" "$@"
        fi
        "#
    }
}

/// Windows Command Prompt shim.
pub fn cmd_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        @SETLOCAL
        @IF EXIST "%~dp0\node.exe" (
            "%~dp0\node.exe" "%~dp0\{relative_file}" %*
        ) ELSE (
        @SET PATHEXT=%PATHEXT:;.JS;=;%
            node "%~dp0\{relative_file}" %*
        )
        "#,
        relative_file = relative_file.replace('/', "\\")
    }
    .replace('\n', "\r\n") // replace \n to \r\n for windows
}

/// `PowerShell` shim.
pub fn pwsh_shim(relative_file: &str) -> String {
    formatdoc! {
        r#"
        #!/usr/bin/env pwsh
        $basedir=Split-Path $MyInvocation.MyCommand.Definition -Parent

        $exe=""
        if ($PSVersionTable.PSVersion -lt "6.0" -or $IsWindows) {{
            # Fix case when both the Windows and Linux builds of Node
            # are installed in the same directory
            $exe=".exe"
        }}
        $ret=0
        if (Test-Path "$basedir/node$exe") {{
            # Support pipeline input
            if ($MyInvocation.ExpectingInput) {{
                $input | & "$basedir/node$exe" "$basedir/{relative_file}" $args
            }} else {{
                & "$basedir/node$exe" "$basedir/{relative_file}" $args
            }}
            $ret=$LASTEXITCODE
        }} else {{
            # Support pipeline input
            if ($MyInvocation.ExpectingInput) {{
                $input | & "node$exe" "$basedir/{relative_file}" $args
            }} else {{
                & "node$exe" "$basedir/{relative_file}" $args
            }}
            $ret=$LASTEXITCODE
        }}
        exit $ret
        "#
    }
}

#[cfg(test)]
#[cfg(not(windows))] // FIXME
mod tests {
    use tempfile::TempDir;
    use tokio::fs::read_to_string;

    use super::*;

    fn format_shim(shim: &str) -> String {
        shim.replace(' ', "·")
    }

    #[test]
    fn test_sh_shim() {
        let shim = sh_shim("pnpm.js");
        // println!("{:#}", format_shim(&shim));
        assert!(shim.contains("pnpm.js"), "{}", format_shim(&shim));
    }

    #[test]
    fn test_cmd_shim() {
        let shim = cmd_shim("yarn.js");
        // println!("{:#?}", format_shim(&shim));
        assert!(shim.contains("yarn.js"), "{}", format_shim(&shim));
        assert!(
            shim.contains("@SETLOCAL\r\n@IF EXIST \"%~dp0\\node.exe\" (\r\n"),
            "{}",
            format_shim(&shim)
        );

        let shim = cmd_shim("../../../../pnpm.js");
        // println!("{:#}", format_shim(&shim));
        assert!(
            shim.contains("node \"%~dp0\\..\\..\\..\\..\\pnpm.js\" %*"),
            "{}",
            format_shim(&shim)
        );
        assert!(
            shim.contains("@SETLOCAL\r\n@IF EXIST \"%~dp0\\node.exe\" (\r\n"),
            "{}",
            format_shim(&shim)
        );
    }

    #[test]
    fn test_pwsh_shim() {
        let shim = pwsh_shim("pnpm.cjs");
        // println!("{:#}", format_shim(&shim));
        assert!(shim.contains("pnpm.cjs"), "{}", format_shim(&shim));
    }

    #[tokio::test]
    async fn test_write_shims_basic() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("node_modules").join(".bin").join("pnpm.js");
        let target = temp_dir.path().join("bin").join("pnpm");

        // Create parent directories
        tokio::fs::create_dir_all(source.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target.parent().unwrap()).await.unwrap();

        // Write shims
        write_shims(&source, &target).await.unwrap();

        // Verify base shim file was created (shell script)
        assert!(target.exists());
        let content = read_to_string(&target).await.unwrap();
        assert!(content.contains("#!/bin/sh"));
        assert!(content.contains("../node_modules/.bin/pnpm.js"));

        // Verify .cmd file was created
        let cmd_file = target.with_extension("cmd");
        assert!(cmd_file.exists());
        let cmd_content = read_to_string(&cmd_file).await.unwrap();
        assert!(cmd_content.contains("@SETLOCAL"));
        assert!(cmd_content.contains("..\\node_modules\\.bin\\pnpm.js"));

        // Verify .ps1 file was created
        let ps1_file = target.with_extension("ps1");
        assert!(ps1_file.exists());
        let ps1_content = read_to_string(&ps1_file).await.unwrap();
        assert!(ps1_content.contains("#!/usr/bin/env pwsh"));
        assert!(ps1_content.contains("../node_modules/.bin/pnpm.js"));
    }

    #[tokio::test]
    async fn test_write_shims_relative_paths() {
        let temp_dir = TempDir::new().unwrap();

        // Test case 1: Source is deeper than target
        let source1 = temp_dir.path().join("deep").join("nested").join("path").join("script.js");
        let target1 = temp_dir.path().join("bin").join("script");

        tokio::fs::create_dir_all(source1.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target1.parent().unwrap()).await.unwrap();

        write_shims(&source1, &target1).await.unwrap();

        let content1 = read_to_string(&target1).await.unwrap();
        assert!(content1.contains("../deep/nested/path/script.js"));

        // Test case 2: Source and target at same level
        let source2 = temp_dir.path().join("scripts").join("tool.js");
        let target2 = temp_dir.path().join("bin").join("tool");

        tokio::fs::create_dir_all(source2.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target2.parent().unwrap()).await.unwrap();

        write_shims(&source2, &target2).await.unwrap();

        let content2 = read_to_string(&target2).await.unwrap();
        assert!(content2.contains("../scripts/tool.js"));
    }

    #[tokio::test]
    async fn test_write_shims_windows_path_conversion() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("node_modules").join("package").join("bin.js");
        let target = temp_dir.path().join("bin").join("package");

        tokio::fs::create_dir_all(source.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target.parent().unwrap()).await.unwrap();

        write_shims(&source, &target).await.unwrap();

        // Check base file (shell script) has forward slashes
        let content = read_to_string(&target).await.unwrap();
        assert!(content.contains("../node_modules/package/bin.js"));

        // Check CMD file has backslashes
        let cmd_file = target.with_extension("cmd");
        let cmd_content = read_to_string(&cmd_file).await.unwrap();
        assert!(cmd_content.contains("..\\node_modules\\package\\bin.js"));
        assert!(!cmd_content.contains("../node_modules/package/bin.js"));

        // Check PS1 file has forward slashes
        let ps1_file = target.with_extension("ps1");
        let ps1_content = read_to_string(&ps1_file).await.unwrap();
        assert!(ps1_content.contains("../node_modules/package/bin.js"));
        assert!(!ps1_content.contains("..\\node_modules\\"));
    }

    #[tokio::test]
    async fn test_write_shims_overwrite_existing() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("src").join("cli.js");
        let target = temp_dir.path().join("bin").join("cli");

        tokio::fs::create_dir_all(source.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target.parent().unwrap()).await.unwrap();

        // Write initial content to files
        tokio::fs::write(&target, "old content").await.unwrap();
        tokio::fs::write(target.with_extension("cmd"), "old cmd content").await.unwrap();
        tokio::fs::write(target.with_extension("ps1"), "old ps1 content").await.unwrap();

        // Write shims (should overwrite)
        write_shims(&source, &target).await.unwrap();

        // Verify files were overwritten
        let content = read_to_string(&target).await.unwrap();
        assert!(!content.contains("old content"));
        assert!(content.contains("../src/cli.js"));

        let cmd_content = read_to_string(target.with_extension("cmd")).await.unwrap();
        assert!(!cmd_content.contains("old cmd content"));
        assert!(cmd_content.contains("@SETLOCAL"));

        let ps1_content = read_to_string(target.with_extension("ps1")).await.unwrap();
        assert!(!ps1_content.contains("old ps1 content"));
        assert!(ps1_content.contains("#!/usr/bin/env pwsh"));
    }

    #[tokio::test]
    async fn test_write_shims_complex_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("a").join("b").join("c").join("script.js");
        let target = temp_dir.path().join("x").join("y").join("z").join("script");

        tokio::fs::create_dir_all(source.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(target.parent().unwrap()).await.unwrap();

        write_shims(&source, &target).await.unwrap();

        // Base file should be shell script with forward slashes
        let content = read_to_string(&target).await.unwrap();
        assert!(content.contains("#!/bin/sh"));
        assert!(content.contains("../../a/b/c/script.js"));

        // CMD file should have backslashes
        let cmd_content = read_to_string(target.with_extension("cmd")).await.unwrap();
        assert!(cmd_content.contains("..\\..\\a\\b\\c\\script.js"));
    }

    #[tokio::test]
    async fn test_sh_shim_content_validation() {
        let shim = sh_shim("lib/cli.js");

        // Verify shebang
        assert!(shim.starts_with("#!/bin/sh"));

        // Verify CYGWIN/MINGW/MSYS handling
        assert!(shim.contains("*CYGWIN*|*MINGW*|*MSYS*)"));
        assert!(shim.contains("cygpath -w"));

        // Verify node execution paths
        assert!(shim.contains("if [ -x \"$basedir/node\" ]"));
        assert!(shim.contains("exec \"$basedir/node\" \"$basedir/lib/cli.js\" \"$@\""));
        assert!(shim.contains("exec node \"$basedir/lib/cli.js\" \"$@\""));
    }

    #[tokio::test]
    async fn test_cmd_shim_content_validation() {
        let shim = cmd_shim("lib/cli.js");

        // Verify Windows batch commands
        assert!(shim.starts_with("@SETLOCAL"));
        assert!(shim.contains("@IF EXIST \"%~dp0\\node.exe\""));
        assert!(shim.contains("\"%~dp0\\node.exe\" \"%~dp0\\lib\\cli.js\" %*"));
        assert!(shim.contains("@SET PATHEXT=%PATHEXT:;.JS;=;%"));
        assert!(shim.contains("node \"%~dp0\\lib\\cli.js\" %*"));

        // Verify line endings are Windows-style
        assert!(shim.contains("\r\n"));
        assert!(!shim.contains("\n\n")); // No double Unix line endings
    }

    #[tokio::test]
    async fn test_pwsh_shim_content_validation() {
        let shim = pwsh_shim("lib/cli.js");

        // Verify shebang
        assert!(shim.starts_with("#!/usr/bin/env pwsh"));

        // Verify PowerShell version handling
        assert!(shim.contains("$PSVersionTable.PSVersion -lt \"6.0\""));
        assert!(shim.contains("$IsWindows"));

        // Verify execution paths
        assert!(shim.contains("Test-Path \"$basedir/node$exe\""));
        assert!(shim.contains("& \"$basedir/node$exe\" \"$basedir/lib/cli.js\" $args"));

        // Verify pipeline input support
        assert!(shim.contains("$MyInvocation.ExpectingInput"));
        assert!(shim.contains("$input |"));

        // Verify exit code handling
        assert!(shim.contains("$ret=$LASTEXITCODE"));
        assert!(shim.contains("exit $ret"));
    }

    #[tokio::test]
    async fn test_write_shims_error_handling() {
        // Test with invalid path (no parent directory)
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.js");
        let target = temp_dir.path().join("non").join("existent").join("path").join("target");

        // This should fail because parent directory doesn't exist
        let result = write_shims(&source, &target).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_shim_path_separator_conversion() {
        // Test forward slashes are converted to backslashes
        let shim = cmd_shim("node_modules/.bin/tool.js");
        assert!(shim.contains("node_modules\\.bin\\tool.js"));
        assert!(!shim.contains("node_modules/.bin/tool.js"));

        // Test multiple levels
        let shim = cmd_shim("a/b/c/d.js");
        assert!(shim.contains("a\\b\\c\\d.js"));
        assert!(!shim.contains("a/b/c/d.js"));
    }

    #[test]
    fn test_relative_path_formats() {
        // Test various relative path formats work correctly
        let paths = vec![
            "../script.js",
            "../../lib/cli.js",
            "../../../node_modules/.bin/tool.js",
            "script.js",
            "./script.js",
        ];

        for path in paths {
            let sh = sh_shim(path);
            assert!(sh.contains(path));

            let ps1 = pwsh_shim(path);
            assert!(ps1.contains(path));

            let cmd = cmd_shim(path);
            let expected_cmd_path = path.replace('/', "\\");
            assert!(cmd.contains(&expected_cmd_path));
        }
    }
}
