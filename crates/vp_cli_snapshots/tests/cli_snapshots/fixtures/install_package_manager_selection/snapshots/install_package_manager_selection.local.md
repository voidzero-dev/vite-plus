# install_package_manager_selection

An unconfigured project offers a package manager selector and installs with the selected manager without pinning it.

## `vp install`

**→ expect-milestone:** `select:package-manager:0`

```

No package manager detected. Please select one:
   Use ↑↓ arrows to navigate, Enter to select, 1-4 for quick selection
   Press Esc, q, or Ctrl+C to cancel installation

  ▶ [1] pnpm (recommended) ←
    [2] npm
    [3] yarn
    [4] bun
```

**← write-key:** `down`

**→ expect-milestone:** `select:package-manager:1`

```

No package manager detected. Please select one:
   Use ↑↓ arrows to navigate, Enter to select, 1-4 for quick selection
   Press Esc, q, or Ctrl+C to cancel installation

    [1] pnpm (recommended)
  ▶ [2] npm ←
    [3] yarn
    [4] bun
```

**← write-key:** `enter`

```

No package manager detected. Please select one:
   Use ↑↓ arrows to navigate, Enter to select, 1-4 for quick selection
   Press Esc, q, or Ctrl+C to cancel installation

    [1] pnpm (recommended)
  ▶ [2] npm ←
    [3] yarn
    [4] bun

✓ Selected package manager: npm

up to date, audited 1 package in <duration>

found 0 vulnerabilities
```

## `vpt stat-file package-lock.json --assert file`

```
package-lock.json: file
```

## `vpt print-file package.json`

```
{"name":"install-package-manager-selection","private":true}
```
