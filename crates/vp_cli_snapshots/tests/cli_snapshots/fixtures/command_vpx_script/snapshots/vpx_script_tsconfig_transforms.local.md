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
<repo>/packages/cli/dist/script-preload.cjs:454
        const result = oxcNode.load(url, context, nextLoad, HELPER_MODULE_NAME);
                               ^

Error: Failed to transform file://<workspace>/plain/decorators.ts: decorators require `"experimentalDecorators": true` in a tsconfig.json that includes this file; standard (TC39) decorators are not supported yet
    at load (<repo>/packages/cli/dist/script-preload.cjs:454:25)
    at nextStep (node:internal/modules/customization_hooks:189:26)
    at loadWithHooks (node:internal/modules/customization_hooks:374:18)
    at #loadSync (node:internal/modules/esm/loader:865:14)
    at ModuleLoader.load (node:internal/modules/esm/loader:824:28)
    at ModuleLoader.loadAndTranslate (node:internal/modules/esm/loader:609:31)
    at #createModuleJob (node:internal/modules/esm/loader:640:36)
    at #getJobFromResolveResult (node:internal/modules/esm/loader:353:34)
    at ModuleLoader.getModuleJobForImport (node:internal/modules/esm/loader:321:41)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:680:25) {
  code: 'GenericFailure'
}

Node.js <version>
```
