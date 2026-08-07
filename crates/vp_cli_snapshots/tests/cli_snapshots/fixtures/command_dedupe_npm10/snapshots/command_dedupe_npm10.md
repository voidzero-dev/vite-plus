# command_dedupe_npm10

## `vp dedupe`

should dedupe dependencies

```

added 3 packages, and audited 4 packages in <duration>

found 0 vulnerabilities
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "npm@10.9.4"
}
```

## `vp dedupe --check`

should check if deduplication would make changes

```

up to date in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "npm@10.9.4"
}
```

## `vp dedupe -- --loglevel=warn`

support pass through arguments

```

up to date, audited 4 packages in <duration>

found 0 vulnerabilities
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "npm@10.9.4"
}
```
