# discover_config_from_subdirectory

## `vpt write-file src/vite.config.ts 'export default {};
'`

```
```

## `cd src && vp lint index.js`

Running from a subdirectory uses the root lint config.

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd src && vp fmt index.js`

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.js`

The root fmt settings apply from a subdirectory too.

```
console.log('hello')
```

## `vpt write-file src/vite.config.ts 'export default { lint: { rules: { '\''no-console'\'': '\''off'\'' } }, fmt: { singleQuote: false, semi: true } };
'`

```
```

## `cd src && vp lint index.js`

Oxlint discovers the lint settings in the working directory.

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd src && vp fmt index.js`

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.js`

Oxfmt discovers the fmt settings in the working directory.

```
console.log("hello");
```

## `vp lint src/index.js`

Running from the root keeps per-file nested lint configs disabled.

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp fmt src/index.js`

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file src/index.js`

Running from the root keeps per-file nested fmt configs disabled.

```
console.log('hello')
```
