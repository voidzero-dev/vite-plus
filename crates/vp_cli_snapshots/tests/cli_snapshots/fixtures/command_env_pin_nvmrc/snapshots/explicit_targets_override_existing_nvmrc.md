# explicit_targets_override_existing_nvmrc

## `vp env pin 22.13.0 --target dev-engines --no-install --force`

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Updated devEngines.runtime in <workspace>/package.json
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc package.json`

```
# Node for local tools and CI
<version> # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true,
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env pin 24.11.0 --target nvmrc --no-install --force`

an explicit .nvmrc target warns about the higher-priority manifest pin

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 24.11.0
  Updated .nvmrc in <workspace>
warn: devEngines.runtime still takes precedence over .nvmrc. Run 'vp env doctor' for details.
note: Version will be downloaded on first use.
```

## `vpt print-file .nvmrc package.json`

```
# Node for local tools and CI
24.11.0 # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true,
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp env unpin --target nvmrc`

```
VITE+ - The Unified Toolchain for the Web

✓ Removed .nvmrc from <workspace>
```

## `vpt stat-file .nvmrc --assert missing`

```
.nvmrc: missing
```

## `vpt print-file package.json`

```
{
  "name": "env-pin-nvmrc",
  "private": true,
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```
