# vpx_script_tsconfig_transforms

tsconfig options that change the transform, and the decorator error.

## `cd jsx-classic && vpx ./main.tsx`

"jsx": "react" with jsxFactory and jsxFragmentFactory

```
classic: <div id="greeting">hello</div> <fragment>fragment child</fragment>
```

## `cd jsx-automatic && vpx ./main.tsx`

"jsx": "react-jsx" with a jsxImportSource mapped by paths

```
automatic: <p>hello</p>
```

## `cd plain && vpx ./main.ts`

Native class fields without legacy decorators; verbatimModuleSyntax keeps the side-effect import

```
side effect kept by verbatimModuleSyntax
native class fields: true 1
```

## `cd plain && vpx ./decorators.ts`

Decorators without experimentalDecorators fail with a clear error

**Exit code:** 1

```

node:internal/modules/run_main:123
    triggerUncaughtException(
    ^
Error: Failed to transform file://<workspace>/plain/decorators.ts: decorators require `"experimentalDecorators": true` in tsconfig.json; standard (TC39) decorators are not supported yet
    at async nextLoad (node:internal/modules/esm/hooks:748:22)
    at async Hooks.load (node:internal/modules/esm/hooks:385:20)
    at async handleMessage (node:internal/modules/esm/worker:199:18) {
  code: 'GenericFailure'
}

Node.js <version>
```
