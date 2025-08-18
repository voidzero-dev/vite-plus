use indoc::formatdoc;

pub fn unix_shim(bin_name: &str) -> String {
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
            exec "$basedir/node"  "$basedir/{bin_name}" "$@"
        else
            exec node  "$basedir/{bin_name}" "$@"
        fi
        "#
    }
}

pub fn win_shim(bin_name: &str) -> String {
    formatdoc! {
        r#"
        @SETLOCAL
        @IF EXIST "%~dp0\node.exe" (
            "%~dp0\node.exe"  "%~dp0\{bin_name}" %*
        ) ELSE (
        @SET PATHEXT=%PATHEXT:;.JS;=;%
            node  "%~dp0\{bin_name}" %*
        )
        "#
    }
}

pub fn power_shell_shim(bin_name: &str) -> String {
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
                $input | & "$basedir/node$exe"  "$basedir/{bin_name}" $args
            }} else {{
                & "$basedir/node$exe"  "$basedir/{bin_name}" $args
            }}
            $ret=$LASTEXITCODE
        }} else {{
            # Support pipeline input
            if ($MyInvocation.ExpectingInput) {{
                $input | & "node$exe"  "$basedir/{bin_name}" $args
            }} else {{
                & "node$exe"  "$basedir/{bin_name}" $args
            }}
            $ret=$LASTEXITCODE
        }}
        exit $ret
        "#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_shim() {
        let shim = unix_shim("pnpm.js");
        assert!(shim.contains("pnpm.js"));
        // println!("{}", shim);
    }

    #[test]
    fn test_win_shim() {
        let shim = win_shim("yarn.js");
        assert!(shim.contains("yarn.js"));
        // println!("{}", shim);
    }

    #[test]
    fn test_power_shell_shim() {
        let shim = power_shell_shim("pnpm.cjs");
        assert!(shim.contains("pnpm.cjs"));
        // println!("{}", shim);
    }
}
