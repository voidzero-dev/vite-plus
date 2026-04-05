# Running Binaries

Use `vpx`, `vp exec`, and `vp dlx` to run binaries without switching between local installs, downloaded packages, and project-specific tools.

## Overview

`vpx` executes a command from a local or remote npm package. It can run a package that is already available locally, download a package on demand, or target an explicit package version.

Use the other binary commands when you need stricter control:

- `vpx` resolves a package binary locally first by default and downloads it if not found; with `pkg@version`, `--package/-p`, or `--shell-mode`, it runs via `vp dlx`
- `vp exec` runs a binary from the current project's `node_modules/.bin`
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
vpx typescript@5.5.4 tsc --version
vpx -p cowsay -c 'echo "hi" | cowsay'
```

## `vp exec`

Use `vp exec` when the binary must come from the current project, for example a binary from a dependency installed in `node_modules/.bin`.

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
vp dlx typescript tsc --version
```
