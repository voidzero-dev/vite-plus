# redaction_selftest

Locks in the redaction guarantees: OS-native path separators normalize to
forward slashes on every platform, byte sizes and content-hash asset
suffixes are masked, and things that must survive (plain 8-letter filename
stems, https:// URLs) survive.

## `vpt print-native-path src/index.ts dist/assets/app.js`

prints OS-native separators; the snapshot must show forward slashes on every OS

```
src/index.ts
dist/assets/app.js
```

## `vpt print 'dist/assets/index-Dra_-aT4.js  0.71 kB / gzip: 0.40 kB / total 1 MB'`

sizes and hash suffixes are masked

```
dist/assets/index-<hash>.js  <size> kB / gzip: <size> kB / total <size> MB
```

## `vpt print 'keep vite-tsconfig.js and https://viteplus.dev/guide/ intact'`

lowercase 8-letter stems and URLs survive redaction

```
keep vite-tsconfig.js and https://viteplus.dev/guide/ intact
```
