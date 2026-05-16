# Coding Agents

This page is a compact command guide for AI coding agents and humans directing them. Read this before automating Vite+ workflows so arguments are sent to the right command and validation uses the Vite+ command surface.

## Quick Rules

- Run `vp help` and `vp help <command>` before guessing command syntax.
- Prefer non-interactive flags in automated runs, such as `--no-interactive`, when a command supports them.
- Keep Vite+ command options before `--`; put template or underlying-tool options after `--` only where the command documents forwarding.
- Prefer `vp check` for local validation loops because it runs the configured format, lint, and type-check workflow together.
- Use `vp run <script-or-task>` for project scripts. Do not call package-manager scripts directly unless you specifically need package-manager behavior.

## Forwarding Arguments

`--` separates Vite+ options from options that should be forwarded to another tool. The most common case is `vp create <template> -- <template-options>`.

For example, the Vite template selector belongs to `create-vite`, not to `vp create` itself:

```bash
vp create vite -- --template react-ts
```

Vite+ options still go before the separator:

```bash
vp create vite --directory apps/web --no-interactive -- --template react-ts
```

If a command page does not document `--` forwarding, do not assume it exists. Check `vp help <command>` or the relevant guide page first.

## Creating Projects

Use [`vp create`](/guide/create) for scaffolding. Agents should choose an explicit template when they already know the target stack.

```bash
# Interactive project creation
vp create

# Create a Vite app and forward the Vite template option
vp create vite -- --template react-ts

# Create from a built-in Vite+ template without prompts
vp create vite:application --directory apps/web --no-interactive

# List built-in and common shorthand templates
vp create --list

# Use a specific entry from a known organization manifest
vp create @your-org:web --no-interactive
```

Common mistakes:

```bash
# Wrong: --template is parsed as a vp create option instead of a template option
vp create vite --template react-ts

# Right: forward --template to create-vite
vp create vite -- --template react-ts

# Wrong: @org without an entry is an interactive picker and exits non-zero in --no-interactive mode
vp create @your-org --no-interactive

# Right: pass a concrete manifest entry when automating org templates
vp create @your-org:web --no-interactive
```

## Migrating Projects

Use [`vp migrate`](/guide/migrate) to move an existing project onto Vite+. For agent-driven migrations, combine the migration guide's prompt with non-interactive execution when appropriate.

```bash
# Migrate the current project
vp migrate

# Migrate without prompts
vp migrate --no-interactive

# Migrate another directory and write agent/editor setup
vp migrate my-app --agent claude --editor vscode
```

After migration, run the normal Vite+ validation loop:

```bash
vp install
vp check
vp test
vp build
```

## Running Tasks

Use [`vp run`](/guide/run) for package scripts and Vite+ task definitions.

```bash
# Run a script or task in the current package
vp run build

# Run a task for a specific workspace package
vp run @my/app#build

# Run recursively across workspace packages
vp run -r build

# Enable task caching for package.json scripts
vp run --cache build
```

Remember that `vp test` is the built-in Vite+ test command, while `vp run test` runs the `test` script from `package.json`.

## Validation Checklist

Before handing work back:

1. Run the command-specific help for anything unfamiliar: `vp help <command>`.
2. Use Vite+ validation commands where possible: `vp check`, `vp test`, and `vp build`.
3. When creating or migrating projects, confirm whether any flags after `--` are intended for the template or underlying tool.
4. Summarize which Vite+ commands were run and whether any manual follow-up remains.

## Related Guides

- [Creating a Project](/guide/create)
- [Migrate to Vite+](/guide/migrate)
- [Run](/guide/run)
- [Check](/guide/check)
- [Commit Hooks and Agent Integration](/guide/commit-hooks)
