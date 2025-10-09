# Fingerprint Ignore Test Fixture

This fixture demonstrates the `fingerprintIgnores` feature for cache fingerprint calculation.

## Task Configuration

The `create-files` task in `vite-task.json` uses the following ignore patterns:

```json
{
  "fingerprintIgnores": [
    "node_modules/**/*",
    "!node_modules/**/package.json",
    "dist/**/*"
  ]
}
```

## Behavior

With these ignore patterns:

1. **`node_modules/**/*`** - Ignores all files under `node_modules/`
2. **`!node_modules/**/package.json`** - BUT keeps `package.json` files (negation pattern)
3. **`dist/**/*`** - Ignores all files under `dist/`

### Cache Behavior

- ✅ Cache **WILL BE INVALIDATED** when `node_modules/pkg-a/package.json` changes
- ❌ Cache **WILL NOT BE INVALIDATED** when `node_modules/pkg-a/index.js` changes
- ❌ Cache **WILL NOT BE INVALIDATED** when `dist/bundle.js` changes

This allows caching package installation tasks where only dependency manifests (package.json) matter for cache validation, not the actual implementation files.

## Example Usage

```bash
# First run - task executes
vite run create-files

# Second run - cache hit (all files tracked in fingerprint remain the same)
vite run create-files

# Modify node_modules/pkg-a/index.js
echo 'modified' > node_modules/pkg-a/index.js

# Third run - still cache hit (index.js is ignored)
vite run create-files

# Modify node_modules/pkg-a/package.json
echo '{"name":"pkg-a","version":"2.0.0"}' > node_modules/pkg-a/package.json

# Fourth run - cache miss (package.json is NOT ignored due to negation pattern)
vite run create-files
```
