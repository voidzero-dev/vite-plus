# command_update_interactive_changelog

## `vp install`


## `vp up --interactive --latest`

interactive updates link directly to the selected version's npmx changelog

**→ expect-milestone:** `multi-select:update:ready`

```
? Choose which dependencies to update ›
⬚ [dependencies] testnpm2 1.0.0 ❯ 1.0.1 https://npmx.dev/package-changelog/testnpm2/v/1.0.1
```

**← write-key:** `space`

**← write-key:** `enter`

```
✔ Choose which dependencies to update · [dependencies] testnpm2 1.0.0 ❯ 1.0.1 https://npmx.dev/package-changelog/testnpm2/v/1.0.1
 -1
-

dependencies:
- testnpm2 1.0.0
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-interactive-changelog",
  "private": true,
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "packageManager": "pnpm@11.0.6"
}
```
