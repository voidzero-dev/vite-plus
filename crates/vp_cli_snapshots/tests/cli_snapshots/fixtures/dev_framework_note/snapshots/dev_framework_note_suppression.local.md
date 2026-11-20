# dev_framework_note_suppression

Explicit Vite arguments, script execution, and packages without a direct framework dependency get no framework hint.

## `vp dev --host 127.0.0.1`

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

## `VP_RUN=1 vp dev`

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

## `npm_lifecycle_event=start vp dev`

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

## `cd plain && vp dev`

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

## `vpt write-file package.json '{"name":"framework-note","type":"module","dependencies":{"astro":"*"},"scripts":{"dev":"astro dev"}}'`


## `vpt replace-file-content vite.config.ts 'export default {' 'export default { clearScreen: false,'`


## `vp dev`

An existing dev script gets only the existing vpr note.

**→ expect-milestone:** `dev-server:ready`

```
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
