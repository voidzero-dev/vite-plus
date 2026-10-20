# vpx_script_jsx_custom

JSX customizations with a runtime installed as a package: classic and automatic runtimes, the development runtime, pragma comments, .jsx with allowJs, and JSX in a CommonJS package.

## `vpt mkdir -p jsx-custom/node_modules`


## `vpt cp -r jsx-custom/mini-jsx jsx-custom/node_modules/mini-jsx`


## `cd jsx-custom/classic-import && vpx ./main.tsx`

"jsx": "react" with jsxFactory and jsxFragmentFactory imported from the package; the import is kept

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/automatic-package && vpx ./main.tsx`

"jsx": "react-jsx" with the package as jsxImportSource

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/automatic-dev && vpx ./main.tsx`

"jsx": "react-jsxdev": jsxDEV from the package's jsx-dev-runtime, with source line numbers

```
<section title="t"><b>bold</b> @2frag @2</section> @1 @2
```

## `cd jsx-custom/pragma-automatic && vpx ./main.tsx`

A @jsxImportSource pragma comment

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/pragma-classic && vpx ./main.tsx`

@jsxRuntime classic, @jsx, and @jsxFrag pragma comments

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/pragma-mixed && vpx ./main.tsx`

Hono's @jsx plus @jsxImportSource pair: the automatic runtime applies and @jsx is ignored

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/jsx-allowjs && vpx ./main.jsx`

A .jsx file gets the tsconfig's JSX options when allowJs is on

```
<section title="t"><b>bold</b>frag</section>
```

## `cd jsx-custom/automatic-cjs && vpx ./main.tsx`

A .tsx in a package without "type" runs as CommonJS and requires the runtime

```
format CommonJS
<section title="t"><b>bold</b>frag</section>
```
