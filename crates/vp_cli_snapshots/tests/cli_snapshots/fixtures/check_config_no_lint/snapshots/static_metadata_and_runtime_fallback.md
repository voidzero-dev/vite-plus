# static_metadata_and_runtime_fallback

## `vp check`

A literal metadata export preserves the configured check phases.

```
note: Lint skipped (check.lint: false in vite.config.ts)
pass: All 3 files are correctly formatted (<duration>, <n> threads)
```

## `vpt write-file vite.config.ts 'export default { check: { fmt: false, lint: false } };
'`


## `vp check`

The next command observes the changed metadata.

**Exit code:** 1

```
note: Format skipped (check.fmt: false in vite.config.ts)
note: Lint skipped (check.lint: false in vite.config.ts)
error: No checks enabled

Enable `lint.options.typeCheck` in vite.config.ts for type-check only, drop a `--no-fmt`/`--no-lint` flag, or re-enable `check.fmt`/`check.lint` in vite.config.ts.
```

## `vpt write-file vite.config.ts 'export default async () => ({ plugins: [{ name: '\''metadata'\'', config() { return { check: { fmt: false, lint: false } }; } }] });
'`


## `vp check`

Config functions and plugin hooks still run through Vite.

**Exit code:** 1

```
note: Format skipped (check.fmt: false in vite.config.ts)
note: Lint skipped (check.lint: false in vite.config.ts)
error: No checks enabled

Enable `lint.options.typeCheck` in vite.config.ts for type-check only, drop a `--no-fmt`/`--no-lint` flag, or re-enable `check.fmt`/`check.lint` in vite.config.ts.
```

## `vpt write-file vite.config.ts 'console.log('\''config side effect'\''); export default { check: { fmt: false, lint: false } };
'`


## `vp check`

A module with executable statements must retain its side effects.

**Exit code:** 1

```
config side effect
note: Format skipped (check.fmt: false in vite.config.ts)
note: Lint skipped (check.lint: false in vite.config.ts)
error: No checks enabled

Enable `lint.options.typeCheck` in vite.config.ts for type-check only, drop a `--no-fmt`/`--no-lint` flag, or re-enable `check.fmt`/`check.lint` in vite.config.ts.
```
