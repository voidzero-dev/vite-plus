# pnpm_publish_json_regression

## `vp pm publish --dry-run --json`

Reject the release range that silently omitted JSON output.

**Exit code:** 1

```
pnpm >= 11.0.0 && < 11.0.7 does not support --json.
```
