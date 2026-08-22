# dev_uses_custom_vite_root

A workspace root remains the app target when its static Vite root points to
the directory that contains index.html. Bare vp dev starts there instead of
eliciting a member package.

## `vp dev --port 12312312312`

**Exit code:** 1

```
error when starting dev server:
Error: No available ports found between 12312312312 and 65535
```
