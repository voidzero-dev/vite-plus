# dev_framework_note_tasks

Configured dev tasks suppress the raw framework command hint, including tasks from dynamic config. Other tasks do not suppress it.

## `vpt write-file app/vite.config.ts 'import base from '\''../vite.config.ts'\'';
export default { ...base, clearScreen: false, run: { tasks: { dev: { command: '\''nuxt dev --host'\'' } } } };
'`


## `vp -C app dev`

A static dev task must not get a suggestion to bypass its command options.

**→ expect-milestone:** `dev-server:ready`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vpt write-file app/vite.config.ts 'import base from '\''../vite.config.ts'\'';
const tasks = { dev: { command: '\''nuxt dev --host'\'' } };
export default { ...base, clearScreen: false, run: { tasks } };
'`


## `vp -C app dev`

A dev task that needs runtime config resolution also suppresses the hint.

**→ expect-milestone:** `dev-server:ready`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vpt write-file app/vite.config.ts 'import base from '\''../vite.config.ts'\'';
export default { ...base, clearScreen: false, run: { tasks: { build: { command: '\''nuxt build'\'' } } } };
'`


## `vp -C app dev`

A build task does not suppress the dev hint.

**→ expect-milestone:** `dev-server:ready`

```
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
