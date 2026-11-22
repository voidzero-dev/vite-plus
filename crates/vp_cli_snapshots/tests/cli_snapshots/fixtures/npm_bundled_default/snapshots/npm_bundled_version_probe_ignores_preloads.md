# npm_bundled_version_probe_ignores_preloads

## `vpt write-file preload.cjs 'console.log('\''preload start'\'');
'`


## `vp pm patch example`

A noisy preload cannot bypass npm's patch version gate

```
warn: npm does not have a 'patch' command.
```

## `vp pm approve-builds`

A noisy preload cannot bypass npm's approval version gate

```
warn: npm runs lifecycle scripts by default. Upgrade to npm >= 11.16.0 for `npm approve-scripts`/`deny-scripts`, or set `ignore-scripts=true` in .npmrc and rebuild approved packages with `vp pm rebuild <package>`.
```

## `vp pm version --json`

The actual npm command still executes the user's preload

```
preload start
{
  "npm": "10.9.9",
  "node": "22.23.4",
  "acorn": "8.16.0",
  "ada": "2.9.2",
  "amaro": "1.1.8",
  "ares": "1.34.9",
  "brotli": "1.1.0",
  "cjs_module_lexer": "2.2.0",
  "cldr": "48.0",
  "icu": "78.3",
  "llhttp": "9.4.3",
  "modules": "127",
  "napi": "10",
  "nbytes": "0.1.3",
  "ncrypto": "0.0.1",
  "nghttp2": "1.69.0",
  "openssl": "3.5.9",
  "simdjson": "4.5.0",
  "simdutf": "6.4.2",
  "sqlite": "3.53.4",
  "tz": "2026e",
  "undici": "6.29.1",
  "unicode": "17.0",
  "uv": "1.51.0",
  "uvwasi": "0.0.23",
  "v8": "12.4.254.21-node.57",
  "zlib": "1.3.1-e00f703",
  "zstd": "1.5.7"
}
```
