# migration_setup_vp_version

## `vpt mkdir -p .github/workflows`


## `vpt cp workflow.txt .github/workflows/ci.yml`


## `vp migrate --no-interactive`

an existing Vite+ project upgrades the frozen setup-vp tag

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite   → <version>
• setup-vp updated to <version> in 1 GitHub Actions file
```

## `vpt print-file .github/workflows/ci.yml`

the workflow uses the exact setup-vp release

```
name: CI

on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: voidzero-dev/setup-vp@<version>
        with:
          cache: true
      - run: vp test
```

## `vp migrate --no-interactive`

the setup-vp migration is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file .github/workflows/ci.yml`

the exact setup-vp release remains unchanged

```
name: CI

on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: voidzero-dev/setup-vp@<version>
        with:
          cache: true
      - run: vp test
```
