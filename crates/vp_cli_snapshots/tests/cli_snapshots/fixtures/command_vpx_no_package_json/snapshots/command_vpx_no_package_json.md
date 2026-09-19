# command_vpx_no_package_json

## `vpx -s cowsay hello`

select a manager without package.json

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

**← write:** `2`

```

No package manager detected. Please select one:
   Use ↑↓ arrows to navigate, Enter to select, 1-4 for quick selection
   Press Esc, q, or Ctrl+C to cancel installation

  ▶ [1] pnpm (recommended) ←
    [2] npm
    [3] yarn
    [4] bun

✓ Selected package manager: npm

 _______
< hello >
 -------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## `vpt stat-file package.json --assert missing`

```
package.json: missing
```
