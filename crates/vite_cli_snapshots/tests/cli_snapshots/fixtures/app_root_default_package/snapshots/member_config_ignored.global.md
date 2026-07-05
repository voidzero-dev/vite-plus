# member_config_ignored

defaultPackage is a root-pointer concept: a member package's own config
declaring it (here pointing at a missing directory) must not redirect or
fail a command already running in that member.

## `cd packages/ui && vp pack`

**Exit code:** 1

```
[1m[31merror:[39m[0m defaultPackage points to a missing directory: ./does-not-exist
```
