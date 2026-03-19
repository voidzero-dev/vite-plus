# Creating a Project

`vp create` interactively scaffolds new Vite+ projects, monorepos, and apps inside existing workspaces.

## Overview

The `create` command is the fastest way to start with Vite+. It can be used in a few different ways:

- Start a new Vite+ monorepo
- Create a new standalone application or library
- Add a new app or library inside an existing project

This command can be used with built-in templates, community templates, or remote GitHub templates.

## Usage

```bash
vp create
vp create <template>
vp create <template> -- <template-options>
```

## Built-in Templates

Vite+ ships with these built-in templates:

- `vite:monorepo` creates a new monorepo
- `vite:application` creates a new application
- `vite:library` creates a new library
- `vite:generator` creates a new generator

## Template Sources

`vp create` is not limited to the built-in templates.

- Use shorthand templates like `vite`, `@tanstack/start`, `svelte`, `next-app`, `nuxt`, `react-router`, and `vue`
- Use full package names like `create-vite` or `create-next-app`
- Use local templates such as `./tools/create-ui-component` or `@acme/generator-*`
- Use remote templates such as `github:user/repo` or `https://github.com/user/template-repo`

Run `vp create --list` to see the built-in templates and the common shorthand templates Vite+ recognizes.

## Options

- `--directory <dir>` writes the generated project into a specific target directory
- `--agent <name>` creates agent instructions files during scaffolding
- `--editor <name>` writes editor config files
- `--hooks` enables pre-commit hook setup
- `--no-hooks` skips hook setup
- `--no-interactive` runs without prompts
- `--verbose` shows detailed scaffolding output
- `--list` prints the available built-in and popular templates

## Template Options

Arguments after `--` are passed directly to the selected template.

This matters when the template itself accepts flags. For example, you can forward Vite template selection like this:

```bash
vp create vite -- --template react-ts
```

## Examples

```bash
# Interactive mode
vp create

# Create a Vite+ monorepo, application, library, or generator
vp create vite:monorepo
vp create vite:application
vp create vite:library
vp create vite:generator

# Use shorthand community templates
vp create vite
vp create @tanstack/start
vp create svelte

# Use full package names
vp create create-vite
vp create create-next-app

# Use remote templates
vp create github:user/repo
vp create https://github.com/user/template-repo
```
