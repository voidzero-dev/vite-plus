# command_pack_css

## `vp pack src/index.ts --minify`

bundles CSS via the bundled @tsdown/css + lightningcss (issue #1586)

```
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ dist/style.css  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt print-file dist/style.css`

lightningcss-optimized output proves @tsdown/css ran

```
.foo {
  color: red;
}

.bar {
  margin: 0;
}
```
