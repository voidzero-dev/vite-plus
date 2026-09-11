# create_skip_hooks_without_git

## `vp create vite:application --directory app --package-manager pnpm --no-agent --no-editor`

declining Git initialization should skip the pre-commit hooks prompt


## `vpt stat-file app/.vite-hooks/pre-commit --assert missing`

pre-commit hooks should not be configured without a Git repository

```
app/.vite-hooks/pre-commit: missing
```
