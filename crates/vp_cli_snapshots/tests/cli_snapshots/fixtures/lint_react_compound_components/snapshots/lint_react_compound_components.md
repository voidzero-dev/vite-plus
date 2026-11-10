# lint_react_compound_components

## `vp lint components.tsx`

allowCompoundComponents: true accepts an object containing only React components

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint mixed.tsx`

the enabled option still rejects an object with a non-component property

**Exit code:** 1

```

  × react(only-export-components): Fast refresh only works when a file only exports components. Use a new file to share constants or functions between components.
   ╭─[mixed.tsx:3:14]
 2 │
 3 │ export const Tag = { Root, size: 1 };
   ·              ───
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt replace-file-content vite.config.ts 'allowCompoundComponents: true' 'allowCompoundComponents: false'`


## `vp lint components.tsx`

allowCompoundComponents: false rejects the same compound component export

**Exit code:** 1

```

  × react(only-export-components): Fast refresh only works when a file only exports components. Move your component(s) to a separate file.
   ╭─[components.tsx:1:7]
 1 │ const Root = () => <div />;
   ·       ────
 2 │ const Label = () => <span />;
   ╰────

  × react(only-export-components): Fast refresh only works when a file only exports components. Move your component(s) to a separate file.
   ╭─[components.tsx:2:7]
 1 │ const Root = () => <div />;
 2 │ const Label = () => <span />;
   ·       ─────
 3 │
   ╰────

Found 0 warnings and 2 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
