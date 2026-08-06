# check_oxlint_env

## `OXLINT_TSGOLINT_PATH=./invalid-path vp lint --type-aware`

should error that ./invalid-path doesn't exist

**Exit code:** 1

```
Failed to find tsgolint executable: OXLINT_TSGOLINT_PATH points to './invalid-path' which does not exist
```
