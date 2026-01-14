# RFC: Split Global CLI

## Background

The global CLI is a single binary that combines all the functionality of the vite-plus toolchain. It is a convenient way to get started with vite-plus, but it is also a large binary that is difficult to maintain.

## Goals

1. Split the global CLI into independent package and reduce size
2. Only include the necessary commands in the global CLI: generate, migration, and package manager commands
3. Delegate all other commands to the local CLI: lint, fmt, build, test, lib, doc, etc.

## User Stories

Install the global CLI first

```bash
npm install -g @voidzero-dev/global
```

### Global CLI Commands

```bash
vite --version
vite --help
vite new --help
```

Generate a new project

```bash
vite new
```

Migrate an existing project

```bash
vite migration
```

Add a package to the project

```bash
vite add vue
```

### Delegate to local CLI Commands

All the other commands are delegated to the local CLI.
If the local CLI is not installed, the global CLI will install it for you.

```bash
vite run build

# if not installed, will install it for you
Add vite-plus as a devDependency in package.json? [y/N]: y

# will install it for you
Installing vite-plus...

# will run the build task
vite run build
```

Format the project

```bash
vite fmt
```

Lint the project

```bash
vite lint
```

Test the project

```bash
vite test
```

Run a build task

```bash
vite run build
```
