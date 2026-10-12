# Running Binaries

Use `vpx`, `vp exec`, and `vp dlx` to run binaries without switching between local installs, downloaded packages, and project-specific tools.

## Overview

`vpx` executes a command from a local or remote npm package. It can run a package that is already available locally, download a package on demand, or target an explicit package version.

Use the other binary commands when you need stricter control:

- `vpx` looks for a binary in local `node_modules/.bin` directories, Vite+-managed global packages, and system `PATH`, in that order, then falls back to `vp dlx`. With `pkg@version`, `--package/-p`, or `--shell-mode`, it runs via `vp dlx` directly.
- `vp exec` runs a command from local `node_modules/.bin` directories, falling back to `PATH` if not found
- `vp dlx` runs a package binary without adding it as a dependency

## `vpx`

Use `vpx` for running any local or remote binary:

```bash
vpx <pkg[@version]> [args...]
```

### Options

- `-p, --package <name>` installs one or more additional packages before running the command
- `-c, --shell-mode` executes the command inside a shell
- `-s, --silent` suppresses Vite+ output and only shows the command output

### Examples

```bash
vpx eslint .
vpx create-vue my-app
vpx oxlint@1.85.0 --version
vpx -p cowsay -c 'echo "hi" | cowsay'
```

## `vp exec`

Use `vp exec` to run tools already installed in your project or available in your environment.

```bash
vp exec <command> [args...]
```

Examples:

```bash
vp exec eslint .
vp exec tsc --noEmit
```

## `vp dlx`

Use `vp dlx` for one-off package execution without adding the package to your project dependencies.

```bash
vp dlx <package> [args...]
```

Examples:

```bash
vp dlx create-vite
vp dlx oxlint@1.85.0 --version
```
