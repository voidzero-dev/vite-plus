# setup_initializes_missing_preferences

## `node verify.mjs preferences`

Setup preserves existing choices and infers missing families before replacing any shims.

```
Missing families: preserve main and fallback aliases, detect external aliases, default to managed.
Repeated setup preserves saved preferences after the external tool disappears.
Explicit mixed preferences, Node mode, and default versions remain unchanged.
Legacy global package ownership is captured before cleanup removes its metadata.
A foreign tool in the shared bin directory remains intact.
Unattended fresh setup with only a Node override initializes every package-manager family.
Shell-only setup does not change preferences.
```
