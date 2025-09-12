# Vite+ Global CLI

- Global `vite` CLI for Vite+
- Everything else is delegated to [vite-plus local CLI][1] for local tasks

## Install

> **Note**: need GitHub token to install before we publish the package to npm.

Add the following lines to your `~/.npmrc` file:

```ini
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
@voidzero-dev:registry=https://npm.pkg.github.com/
```

Create a fine-grained personal access token with the `read:packages` scope. Follow this guide: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens

Use this token to install the global cli:

```bash
GITHUB_TOKEN=<your-token> npm install -g @voidzero-dev/global
```

Use 1Password cli:

```bash
GITHUB_TOKEN=$(op read "op://YOUR_GITHUB_TOKEN_PATH") npm install -g @voidzero-dev/global
```

## Overview

```bash
$ vite --help                                                                        
Usage: vite [OPTIONS] [TASK] [-- <TASK_ARGS>...] [COMMAND]

Commands:
  run      
  lint     
  fmt      
  build    
  test     
  install  
  help     Print this message or the help of the given subcommand(s)

Arguments:
  [TASK]          
  [TASK_ARGS]...  Optional arguments for the tasks, captured after '--'

Options:
  -d, --debug     Display cache for debugging
      --no-debug  
  -h, --help      Print help
  -V, --version   Print version
```

## Commands Usage

### built-in commands: `lint`, `build`, `test`

Execute our own toolchain in current directory, see [vite-plus local CLI][1] for more details.

### task runner

`vite run [name]` runs script with the same `name` from `package.json` across the monorepo (topologically sorted).

e.g.:

```json
// package.json
{
  "scripts": {
    "ready": "vite lint && vite run -r build && vite test"
  },
  "devDependencies": {
    "@voidzero-dev/vite-plus": "*"
  }
}
```

Run the `ready` task with global CLI `vite`:

```bash
vite run ready
```

## Display tracing logs

You can use the `VITE_LOG` environment variable to display tracing logs.

```bash
# display trace level logs
VITE_LOG=trace vite run ready

# display debug level logs
VITE_LOG=debug vite run ready
```

## Development

- The global executable is `vite`, use `vite-dev` for development

Example workflow:

1. Make `vite` (and `vite-dev`) available globally:

```sh
cd packages/global
pnpm link
pnpm dev
```

2. From `vite-plus` package, link `multiplexer` package and use `vite-plus` in
   any project's `package.json`:

```sh
cd packages/cli
pnpm link ../multiplexer/
pnpm dev
```

3. Build multiplexer

```sh
cd packages/multiplexer
pnpm dev
```

4. Install in project

Use `vite new` anywhere, or run this directly inside this repo:

```sh
cd packages/global/templates/minimal
pnpm link ../../../cli/
```

Outside this repo do `pnpm link to/vite-plus/packages/cli/`

5. Run tasks

Now the following commands all do the same thing:

```sh
vite lint
pnpm vite-plus lint
```

Or use the task runner for

```sh
vite task build lint
pnpm vite-plus task build lint
pnpm run all
```

## Verdaccio

Install [Verdaccio][2] for local actual package installs ([pkg.pr.new][3]
publishes only from CI and e.g. `npm link` doesn't always cut it).

[1]: ../cli/README.md
[2]: ./verdaccio.md
[3]: https://github.com/stackblitz-labs/pkg.pr.new
