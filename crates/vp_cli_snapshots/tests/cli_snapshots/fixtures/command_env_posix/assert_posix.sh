. ./env || {
    echo "failed to source env"
    exit 1
}

if [ "$VP_HOME" != "$EXPECTED_VP_HOME" ]; then
    echo "VP_HOME mismatch: expected $EXPECTED_VP_HOME, got $VP_HOME"
    exit 1
fi

expected_bin="$EXPECTED_VP_HOME/bin"
first_path_entry=${PATH%%:*}
if [ "$first_path_entry" != "$expected_bin" ]; then
    echo "PATH mismatch: expected first entry $expected_bin, got $first_path_entry"
    exit 1
fi

bin_count=0
remaining_path=$PATH
while :; do
    case $remaining_path in
        *:*)
            path_entry=${remaining_path%%:*}
            remaining_path=${remaining_path#*:}
            ;;
        *)
            path_entry=$remaining_path
            remaining_path=
            ;;
    esac
    if [ "$path_entry" = "$expected_bin" ]; then
        bin_count=$((bin_count + 1))
    fi
    if [ -z "$remaining_path" ]; then
        break
    fi
done
if [ "$bin_count" -ne 1 ]; then
    echo "PATH contains the Vite+ bin directory $bin_count times"
    exit 1
fi

command -v vp >/dev/null || {
    echo "env did not define the vp wrapper"
    exit 1
}

vp_output=$(vp --version) || {
    echo "vp --version failed through the POSIX wrapper"
    exit 1
}
if [ -z "$vp_output" ]; then
    echo "vp --version returned no output"
    exit 1
fi

VP_NODE_VERSION=18.20.0
export VP_NODE_VERSION
vp env use --help >/dev/null || {
    echo "vp env use --help failed through the POSIX wrapper"
    exit 1
}
if [ "$VP_NODE_VERSION" != "18.20.0" ]; then
    echo "vp env use --help changed VP_NODE_VERSION"
    exit 1
fi

vp env use 20.18.0 --no-install || {
    echo "vp env use failed through the POSIX wrapper"
    exit 1
}
if [ "${VP_NODE_VERSION:-}" != "20.18.0" ]; then
    echo "VP_NODE_VERSION mismatch: expected 20.18.0, got ${VP_NODE_VERSION:-<unset>}"
    exit 1
fi

vp env use --unset || {
    echo "vp env use --unset failed through the POSIX wrapper"
    exit 1
}
if [ "${VP_NODE_VERSION+x}" = x ]; then
    echo "vp env use --unset did not remove VP_NODE_VERSION"
    exit 1
fi

vp env use --no-install || {
    echo "vp env use without a version failed through the POSIX wrapper"
    exit 1
}
if [ "${VP_NODE_VERSION:-}" != "22.18.0" ]; then
    echo "file-based VP_NODE_VERSION mismatch: expected 22.18.0, got ${VP_NODE_VERSION:-<unset>}"
    exit 1
fi

if [ -n "${BASH_VERSION:-}" ]; then
    complete -p vp >/dev/null || {
        echo "env did not register Bash completions"
        exit 1
    }
elif [ -n "${ZSH_VERSION:-}" ]; then
    whence -w _vpr_complete >/dev/null || {
        echo "env did not register Zsh completions"
        exit 1
    }
fi

echo "POSIX environment checks passed ($SHELL_LABEL)"
