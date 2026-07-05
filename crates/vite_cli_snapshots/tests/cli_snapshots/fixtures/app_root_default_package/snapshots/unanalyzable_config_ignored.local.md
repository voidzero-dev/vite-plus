# unanalyzable_config_ignored

Regression guard for spread/unanalyzable configs: a config that only parses
as an open map might hide defaultPackage behind the spread, but that must
not fail the command. Bare vp build falls through and runs in place.

## `cd spread && vp build`

```
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
