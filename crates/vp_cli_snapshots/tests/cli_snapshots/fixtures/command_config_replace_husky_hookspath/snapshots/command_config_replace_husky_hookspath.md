# command_config_replace_husky_hookspath

## `git init`


## `git config core.hooksPath .husky/_`


## `vp config --no-agent`

should preserve the existing Husky hooks path

```
core.hooksPath is already set to ".husky/_", skipping
```

## `git config --local core.hooksPath`

should remain .husky/_

```
.husky/_
```
