# shell_integration_cwd_templates

## `VP_HOME=${workspace}/home vp env setup --refresh`


## `vpt print-file home/env`

POSIX wrapper and zsh vpr completion keep global -C before env use/run

```
#!/bin/sh
# Vite+ environment setup (https://viteplus.dev)
export VP_HOME="<workspace>/home"
__vp_bin="<workspace>/home/bin"
while case ":${PATH}:" in *":${__vp_bin}:"*) true ;; *) false ;; esac; do
    __vp_tmp=":${PATH}:"
    __vp_before="${__vp_tmp%%":${__vp_bin}:"*}"
    __vp_before="${__vp_before#:}"
    __vp_after="${__vp_tmp#*":${__vp_bin}:"}"
    __vp_after="${__vp_after%:}"
    PATH="${__vp_before}${__vp_before:+${__vp_after:+:}}${__vp_after}"
done
export PATH="${__vp_bin}${PATH:+:${PATH}}"
unset __vp_bin __vp_tmp __vp_before __vp_after

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
vp() {
    __vp_env_use=
    if [ "${1-}" = "env" ] && [ "${2-}" = "use" ]; then
        __vp_env_use=1
    elif [ "${1-}" = "-C" ] && [ "${3-}" = "env" ] && [ "${4-}" = "use" ]; then
        __vp_env_use=1
    else
        case "${1-}" in
            -C?*)
                if [ "${2-}" = "env" ] && [ "${3-}" = "use" ]; then
                    __vp_env_use=1
                fi
                ;;
        esac
    fi

    if [ -n "$__vp_env_use" ]; then
        unset __vp_env_use
        case " $* " in *" -h "*|*" --help "*) command vp "$@"; return; esac
        __vp_out="$(VP_ENV_USE_EVAL_ENABLE=1 VP_SHELL=sh command vp "$@")" || return $?
        eval "$__vp_out"
    else
        unset __vp_env_use
        command vp "$@"
    fi
}

# Dynamic shell completion for bash/zsh
if [ -n "${BASH_VERSION-}" ] && type complete >/dev/null 2>&1; then
    # Bash invoked as `sh` rejects process substitution in the generated script.
    case ":${SHELLOPTS-}:" in
        *:posix:*) ;;
        *)
            eval "$(command vp __complete_script__ --shell bash)"
            eval "$(command vp __complete_script__ --shell bash --alias vpr)"
            ;;
    esac
elif [ -n "${ZSH_VERSION-}" ] && type compdef >/dev/null 2>&1; then
    eval "$(command vp __complete_script__ --shell zsh)"
    eval "$(command vp __complete_script__ --shell zsh --alias vpr)"
fi
```

## `vpt print-file home/env.fish`

fish wrapper and vpr completion keep global -C before env use/run

```
# Vite+ environment setup (https://viteplus.dev)
set -gx VP_HOME "<workspace>/home"
while set -l __vp_idx (contains -i -- "<workspace>/home/bin" $PATH)
    set -e PATH[$__vp_idx]
end
set -gx PATH "<workspace>/home/bin" $PATH

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
function vp
    set -l __vp_command_index 1
    if test (count $argv) -ge 1
        if test "$argv[1]" = "-C"
            set __vp_command_index 3
        else if string match -qr '^-C.+' -- "$argv[1]"
            set __vp_command_index 2
        end
    end
    set -l __vp_next_index (math $__vp_command_index + 1)

    if test (count $argv) -ge $__vp_next_index; and test "$argv[$__vp_command_index]" = "env"; and test "$argv[$__vp_next_index]" = "use"
        if contains -- -h $argv; or contains -- --help $argv
            command vp $argv; return
        end
        set -lx VP_ENV_USE_EVAL_ENABLE 1
        set -lx VP_SHELL fish
        set -l __vp_out (command vp $argv); or return $status
        for __vp_command in $__vp_out
            eval $__vp_command; or return $status
        end
        return 0
    else
        command vp $argv
    end
end

# Dynamic shell completion for fish
command vp __complete_script__ --shell fish | source
command vp __complete_script__ --shell fish --alias vpr | source
```

## `vpt print-file home/env.nu`

Nushell wrapper and vpr completion keep global -C before env use/run

```
# Vite+ environment setup (https://viteplus.dev)
$env.VP_HOME = ("<workspace>/home" | path expand --no-symlink)
$env.PATH = ($env.PATH | where { $in != "<workspace>/home/bin" } | prepend "<workspace>/home/bin")

# Shell function wrapper: intercepts `vp env use` to parse its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
def --env --wrapped vp [...args: string@"nu-complete vp"] {
    let command_args = if ($args | length) >= 2 and $args.0 == "-C" {
        $args | skip 2
    } else if ($args | length) >= 1 and ($args.0 | str starts-with "-C") and $args.0 != "-C" {
        $args | skip 1
    } else {
        $args
    }
    if ($command_args | length) >= 2 and $command_args.0 == "env" and $command_args.1 == "use" {
        if ("-h" in $args) or ("--help" in $args) {
            ^vp ...$args
            return
        }
        let out = (with-env { VP_ENV_USE_EVAL_ENABLE: "1", VP_SHELL: "nu" } {
            ^vp ...$args
        })
        let lines = ($out | lines)
        let exports = ($lines | where { $in =~ '^\$env\.' } | parse '$env.{key} = "{value}"')
        let export_keys = ($exports | get key? | default [])
        # Exclude keys that also appear in exports: when vp emits `hide-env X` then
        # `$env.X = "v"` (e.g. `vp env use` with no args resolving from .node-version),
        # the set should win.
        let unsets = ($lines | where { $in =~ '^hide-env ' } | parse 'hide-env {key}' | get key? | default [] | where { $in not-in $export_keys })
        if ($exports | is-not-empty) {
            load-env ($exports | reduce -f {} {|it, acc| $acc | insert $it.key $it.value})
        }
        for key in $unsets {
            if ($key in $env) { hide-env $key }
        }
    } else {
        ^vp ...$args
    }
}

# Shell completion for nushell
def "nu-complete vp" [context: string] {
    let out = (^vp __complete_word__ --shell nu --line $context | complete)
    if $out.exit_code != 0 { return null }
    let lines = ($out.stdout | lines | where {|line| $line != "" })
    let marker = "\u{1}"
    let wants_files = ($lines | any {|line| $line == $marker + "files" or $line == $marker + "dirs" or $line == $marker + "executables" or $line == $marker + "commands" })
    let candidates = ($lines | where {|line| not ($line | str starts-with $marker) } | each {|line|
        let parts = ($line | split row (char tab))
        { value: ($parts | get 0), description: (if ($parts | length) > 1 { $parts | get 1 } else { "" }) }
    })
    if ($candidates | is-empty) and $wants_files { null } else { $candidates }
}
# vpr uses the Vite Task executable view.
def "nu-complete vpr" [context: string] {
    nu-complete vp $context
}
export extern "vpr" [...args: string@"nu-complete vpr"]
```

## `vpt print-file home/env.ps1`

PowerShell wrapper and vpr completion keep global -C before env use/run

```
# Vite+ environment setup (https://viteplus.dev)
$env:VP_HOME = '<workspace>/home'
$__vp_bin = '<workspace>/home/bin'
if ($env:Path -split ';' -notcontains $__vp_bin) {
    $env:Path = "$__vp_bin;$env:Path"
}

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
function vp {
    $__vp_command_index = 0
    if ($args.Count -ge 1) {
        if ($args[0] -eq "-C") {
            $__vp_command_index = 2
        } elseif ("$($args[0])" -like "-C?*") {
            $__vp_command_index = 1
        }
    }
    if ($args.Count -ge ($__vp_command_index + 2) -and $args[$__vp_command_index] -eq "env" -and $args[$__vp_command_index + 1] -eq "use") {
        if ($args -contains "-h" -or $args -contains "--help") {
            & (Join-Path $__vp_bin "vp") @args; return
        }
        $env:VP_ENV_USE_EVAL_ENABLE = "1"
        $env:VP_SHELL = "pwsh"
        $output = & (Join-Path $__vp_bin "vp") @args 2>&1 | ForEach-Object {
            if ($_ -is [System.Management.Automation.ErrorRecord]) {
                Write-Host $_.Exception.Message
            } else {
                $_
            }
        }
        Remove-Item Env:VP_ENV_USE_EVAL_ENABLE -ErrorAction SilentlyContinue
        Remove-Item Env:VP_SHELL -ErrorAction SilentlyContinue
        if ($LASTEXITCODE -eq 0 -and $output) {
            Invoke-Expression ($output -join "`n")
        }
    } else {
        & (Join-Path $__vp_bin "vp") @args
    }
}

# Dynamic shell completion for PowerShell
& (Join-Path $__vp_bin "vp") __complete_script__ --shell powershell | Out-String | Invoke-Expression
& (Join-Path $__vp_bin "vp") __complete_script__ --shell powershell --alias vpr | Out-String | Invoke-Expression
```
