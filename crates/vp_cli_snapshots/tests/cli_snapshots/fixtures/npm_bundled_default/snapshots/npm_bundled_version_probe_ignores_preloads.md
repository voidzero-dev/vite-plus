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
  "npm": "10.9.3",
  "node": "22.18.0",
  "acorn": "8.15.0",
  "ada": "2.9.2",
  "amaro": "1.1.0",
  "ares": "1.34.5",
  "brotli": "1.1.0",
  "cjs_module_lexer": "2.1.0",
  "cldr": "47.0",
  "icu": "77.1",
  "llhttp": "9.3.0",
  "modules": "127",
  "napi": "10",
  "nbytes": "0.1.1",
  "ncrypto": "0.0.1",
  "nghttp2": "1.64.0",
  "openssl": "3.0.16",
  "simdjson": "3.13.0",
  "simdutf": "6.4.2",
  "sqlite": "3.50.2",
  "tz": "2025b",
  "undici": "6.21.2",
  "unicode": "16.0",
  "uv": "1.51.0",
  "uvwasi": "0.0.21",
  "v8": "12.4.254.21-node.27",
  "zlib": "1.3.1-470d3a2",
  "zstd": "1.5.7"
}
```
