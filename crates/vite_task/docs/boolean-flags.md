# Boolean Flags in vite-plus

This document describes how boolean flags work in vite-plus commands.

## Negation Pattern

All boolean flags in vite-plus support a negation pattern using the `--no-` prefix. When a `--no-*` flag is used, it explicitly sets the corresponding boolean option to `false`.

## Available Boolean Flags

### Global Flags

- `--debug` / `--no-debug` - Enable or disable cache debugging output
  - Short form: `-d` (only for positive form)

### Run Command Flags

- `--recursive` / `--no-recursive` - Enable or disable recursive task execution across all packages
  - Short form: `-r` (only for positive form)

- `--parallel` / `--no-parallel` - Enable or disable parallel task execution
  - Short form: `-p` (only for positive form)

- `--sequential` / `--no-sequential` - Enable or disable sequential task execution
  - Short form: `-s` (only for positive form)

- `--topological` / `--no-topological` - Enable or disable topological ordering based on package dependencies
  - Short form: `-t` (only for positive form)

## Behavior

### Conflicts

The positive and negative forms of a flag are mutually exclusive. You cannot use both `--flag` and `--no-flag` in the same command:

```bash
# This will result in an error
vite run --recursive --no-recursive build
```

### Precedence

When only the negative form is used, it takes precedence and explicitly sets the value to `false`:

```bash
# Explicitly disable topological ordering
vite run build -r --no-topological
```

### Default Values

The negative flags are particularly useful for overriding default behaviors:

- `--recursive` with `--no-topological`: By default, recursive runs enable topological ordering. Use `--no-topological` to disable it:
  ```bash
  # Recursive run WITHOUT topological ordering
  vite run build -r --no-topological
  ```

## Examples

```bash
# Run with debugging disabled (useful if debug is enabled by default in config)
vite --no-debug build

# Recursive build without topological ordering
vite run build --recursive --no-topological

# Explicitly disable parallel execution
vite run build --no-parallel

# Run tests sequentially, not in parallel
vite run test --no-parallel
```

## Implementation Details

The `--no-*` flags use clap's `conflicts_with` attribute to ensure they cannot be used together with their positive counterparts. When processing flags, vite-plus uses a `resolve_bool_flag` function that gives precedence to the negative form when present.

This pattern provides a consistent and intuitive way to explicitly disable features that might be enabled by default or through configuration files.
