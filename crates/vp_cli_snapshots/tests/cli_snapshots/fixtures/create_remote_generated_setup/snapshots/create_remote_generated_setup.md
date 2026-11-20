# create_remote_generated_setup

## `vp create vp-test-generated-setup --package-manager pnpm -- my-app`

after the create CLI: no git prompt, a hooks prompt, then append/skip for the generated AGENTS.md


## `vpt grep-file my-app/AGENTS.md '<'\!'--VITE PLUS START-->'`

Vite+ instructions are appended to the generated AGENTS.md

```
my-app/AGENTS.md: found "<!--VITE PLUS START-->"
```

## `vpt print-file my-app/.vscode/settings.json`

generated editor config is merged without a prompt

```
{
  "editor.tabSize": 2,
  "npm.scriptRunner": "vp",
  "editor.defaultFormatter": "oxc.oxc-vscode",
  "[javascript]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[javascriptreact]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[typescript]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "oxc.disableNestedConfig": true,
  "oxc.fmt.disableNestedConfig": true,
  "editor.formatOnSave": true,
  "editor.formatOnSaveMode": "file",
  "editor.codeActionsOnSave": {
    "source.fixAll.oxc": "explicit"
  }
}
```

## `vpt stat-file my-app/.vite-hooks/pre-commit --assert file`

hooks are set up in the repository the create CLI initialized

```
my-app/.vite-hooks/pre-commit: file
```
