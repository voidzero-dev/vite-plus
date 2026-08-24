# new_create_vite

## `vp create vite:application --no-interactive --git --editor vscode`

create vite app with default values


## `vpt list-dir vite-plus-application/package.json`

check package.json

```
vite-plus-application/package.json
```

## `vpt stat-file vite-plus-application/.vscode/settings.json --assert file`

check VS Code settings created

```
vite-plus-application/.vscode/settings.json: file
```

## `vpt stat-file vite-plus-application/.vscode/extensions.json --assert file`

check VS Code extensions created

```
vite-plus-application/.vscode/extensions.json: file
```

## `node check-trackable.cjs vite-plus-application .vscode/settings.json`

check VS Code settings are trackable

```
.vscode/settings.json trackable
```

## `node check-trackable.cjs vite-plus-application .vscode/extensions.json`

check VS Code extensions are trackable

```
.vscode/extensions.json trackable
```

## `vpt stat-file vite-plus-application/.github/workflows/copilot-setup-steps.yml --assert-not file`

default create should not add Copilot setup workflow

```
vite-plus-application/.github/workflows/copilot-setup-steps.yml: missing
```

## `vp create vite:application --no-interactive --directory claude-app --agent claude`

create vite app with non-Copilot agent


## `vpt stat-file claude-app/.github/workflows/copilot-setup-steps.yml --assert-not file`

non-Copilot agent should not add Copilot setup workflow

```
claude-app/.github/workflows/copilot-setup-steps.yml: missing
```

## `vp create vite:application --no-interactive --directory no-agent-app --no-agent`

create vite app without agent setup


## `vpt stat-file no-agent-app/.github/workflows/copilot-setup-steps.yml --assert-not file`

--no-agent should not add Copilot setup workflow

```
no-agent-app/.github/workflows/copilot-setup-steps.yml: missing
```

## `vp create vite:application --no-interactive --directory copilot-app --agent copilot`

create vite app with Copilot agent setup


## `vpt print-file copilot-app/.github/workflows/copilot-setup-steps.yml`

check Copilot setup workflow

```
name: "Copilot Setup Steps"

on:
  workflow_dispatch:
  push:
    paths:
      - .github/workflows/copilot-setup-steps.yml
  pull_request:
    paths:
      - .github/workflows/copilot-setup-steps.yml

jobs:
  copilot-setup-steps:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - name: Checkout code
        uses: actions/checkout@v6
        with:
          persist-credentials: false
      - name: Set up Vite+
        uses: voidzero-dev/setup-vp@v1
        with:
          cache: true
          run-install: true
      - name: Verify Vite+
        run: vp --version
```

## `vp create vite:application --no-interactive --directory my-react-ts -- --template react-ts`

create vite app with react-ts template


## `vpt list-dir my-react-ts/package.json`

check package.json

```
my-react-ts/package.json
```
