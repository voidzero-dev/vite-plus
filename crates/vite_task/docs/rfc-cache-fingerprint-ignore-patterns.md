# RFC: Vite+ Cache Fingerprint Ignore Patterns

## Summary

Add support for glob-based ignore patterns to the cache fingerprint calculation, allowing tasks to exclude specific files/directories from triggering cache invalidation while still including important files within ignored directories.

## Motivation

Current cache fingerprint behavior tracks all files accessed during task execution. This causes unnecessary cache invalidation in scenarios like:

1. **Package installation tasks**: The `node_modules` directory changes frequently, but only `package.json` files within it are relevant for cache validation
2. **Build output directories**: Generated files in `dist/` or `.next/` that should not invalidate the cache
3. **Large dependency directories**: When only specific files within large directories matter for reproducibility

### Example Use Case

For an `install` task that runs `pnpm install`:

- Changes to `node_modules/**/*/index.js` should NOT invalidate the cache
- Changes to `node_modules/**/*/package.json` SHOULD invalidate the cache
- This allows cache hits when dependencies remain the same, even if their internal implementation files have different timestamps or minor variations

## Proposed Solution

### Configuration Schema

Extend `TaskConfig` in `vite-task.json` to support a new optional field `fingerprintIgnores`:

```json
{
  "tasks": {
    "my-task": {
      "command": "echo bar",
      "cacheable": true,
      "fingerprintIgnores": [
        "node_modules/**/*",
        "!node_modules/**/*/package.json"
      ]
    }
  }
}
```

### Ignore Pattern Syntax

The ignore patterns follow standard glob syntax with gitignore-style semantics:

1. **Basic patterns**:
   - `node_modules/**/*` - ignore all files under node_modules
   - `dist/` - ignore the dist directory
   - `*.log` - ignore all log files

2. **Negation patterns** (prefixed with `!`):
   - `!node_modules/**/*/package.json` - include package.json files even though node_modules is ignored
   - `!important.log` - include important.log even though *.log is ignored

3. **Pattern evaluation order**:
   - Patterns are evaluated in order
   - Later patterns override earlier ones
   - Negation patterns can "un-ignore" files matched by earlier patterns
   - Last match wins semantics

### Implementation Details

#### 1. Configuration Schema Changes

**File**: `crates/vite_task/src/config/mod.rs`

```rust
pub struct TaskConfig {
    // ...

    // New field
    #[serde(default)]
    pub(crate) fingerprint_ignores: Option<Vec<Str>>,
}
```

#### 2. Fingerprint Validation Changes

**File**: `crates/vite_task/src/fingerprint.rs`

Modify `PostRunFingerprint::create()` to filter paths based on ignore patterns:

```rust
impl PostRunFingerprint {
    pub fn create(
        executed_task: &ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
        fingerprint_ignores: Option<&[Str]>,  // New parameter
    ) -> Result<Self, Error> {
        let ignore_matcher = fingerprint_ignores
            .filter(|patterns| !patterns.is_empty())
            .map(GlobPatternSet::new)
            .transpose()?;

        let inputs = executed_task
            .path_reads
            .par_iter()
            .filter(|(path, _)| {
                if let Some(ref matcher) = ignore_matcher {
                    !matcher.is_match(path)
                } else {
                    true
                }
            })
            .flat_map(|(path, path_read)| {
                Some((|| {
                    let path_fingerprint =
                        fs.fingerprint_path(&base_dir.join(path).into(), *path_read)?;
                    Ok((path.clone(), path_fingerprint))
                })())
            })
            .collect::<Result<HashMap<RelativePathBuf, PathFingerprint>, Error>>()?;
        Ok(Self { inputs })
    }
}
```

#### 3. Cache Update Integration

**File**: `crates/vite_task/src/cache.rs`

Update `CommandCacheValue::create()` to pass ignore patterns:

```rust
impl CommandCacheValue {
    pub fn create(
        executed_task: ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
        fingerprint_ignores: Option<&[Str]>,  // New parameter
    ) -> Result<Self, Error> {
        let post_run_fingerprint = PostRunFingerprint::create(
            &executed_task,
            fs,
            base_dir,
            fingerprint_ignores,
        )?;
        Ok(Self {
            post_run_fingerprint,
            std_outputs: executed_task.std_outputs,
            duration: executed_task.duration,
        })
    }
}
```

### Performance Considerations

1. **Pattern compilation**: Glob patterns are compiled once when loading the task configuration
2. **Filtering overhead**: Path filtering happens during fingerprint creation (only when caching)
3. **Memory impact**: Minimal - only stores compiled glob patterns per task
4. **Parallel processing**: Existing parallel iteration over paths is preserved

### Edge Cases

1. **Empty ignore list**: No filtering applied (backward compatible)
2. **Conflicting patterns**: Later patterns take precedence
3. **Invalid glob syntax**: Return error during workspace loading
4. **Absolute paths in patterns**: Treated as relative to package directory
5. **Directory vs file patterns**: Both supported via glob syntax

## Alternative Designs Considered

### Alternative 1: `inputs` field extension

Extend the existing `inputs` field to support ignore patterns:

```json
{
  "inputs": {
    "include": ["src/**/*"],
    "exclude": ["src/**/*.test.js"]
  }
}
```

**Rejected because**:

- The `inputs` field currently uses a different mechanism (pre-execution declaration)
- This feature is about post-execution fingerprint filtering
- Mixing the two concepts would be confusing

### Alternative 2: Separate `fingerprintExcludes` field

Only support exclude patterns (no negation):

```json
{
  "fingerprintExcludes": ["node_modules/**/*"]
}
```

**Rejected because**:

- Cannot express "ignore everything except X" patterns
- Less flexible for complex scenarios
- Gitignore-style syntax is more familiar to developers

### Alternative 3: Include/Exclude separate fields

```json
{
  "fingerprintExcludes": ["node_modules/**/*"],
  "fingerprintIncludes": ["node_modules/**/*/package.json"]
}
```

**Rejected because**:

- More verbose
- Less clear precedence rules
- Gitignore-style is a proven pattern

## Migration Path

### Backward Compatibility

This feature is fully backward compatible:

- Existing task configurations work unchanged
- Default value for `fingerprintIgnores` is `None` (when omitted)
- No behavior changes when field is absent or `null`
- Empty array `[]` is treated the same as `None` (no filtering)

## Testing Strategy

### Unit Tests

1. **Pattern matching**:
   - Test glob pattern compilation
   - Test negation pattern precedence
   - Test edge cases (empty patterns, invalid syntax)

2. **Fingerprint filtering**:
   - Test path filtering with various patterns
   - Test no filtering when patterns are empty
   - Test complex pattern combinations

3. **Cache behavior**:
   - Test cache hit when ignored files change
   - Test cache miss when non-ignored files change
   - Test negation patterns work correctly

### Integration Tests

Create fixtures with realistic scenarios:

```
fixtures/fingerprint-ignore-test/
  package.json
  vite-task.json  # with fingerprintIgnores config
  node_modules/
    pkg-a/
      package.json
      index.js
    pkg-b/
      package.json
      index.js
```

Test cases:

1. Cache hits when `index.js` files change
2. Cache misses when `package.json` files change
3. Negation patterns correctly include files

## Documentation Requirements

### User Documentation

Add to task configuration docs:

````markdown
### fingerprintIgnores

Type: `string[]`
Default: `[]`

Glob patterns to exclude files from cache fingerprint calculation.
Patterns starting with `!` are negation patterns that override earlier excludes.

Example:

```json
{
  "tasks": {
    "install": {
      "command": "pnpm install",
      "cacheable": true,
      "fingerprintIgnores": [
        "node_modules/**/*",
        "!node_modules/**/*/package.json"
      ]
    }
  }
}
```
````

This configuration ignores all files in `node_modules` except `package.json`
files, which are still tracked for cache validation.

````
### Examples Documentation

Add common patterns:

1. **Package installation**:
   ```json
   "fingerprintIgnores": [
     "node_modules/**/*",
     "!node_modules/**/*/package.json",
     "!node_modules/.pnpm/lock.yaml"
   ]
````

2. **Build outputs**:
   ```json
   "fingerprintIgnores": [
     "dist/**/*",
     ".next/**/*",
     "build/**/*"
   ]
   ```

3. **Temporary files**:
   ```json
   "fingerprintIgnores": [
     "**/*.log",
     "**/.DS_Store",
     "**/tmp/**"
   ]
   ```

## Conclusion

This RFC proposes adding glob-based ignore patterns to cache fingerprint calculation. The feature:

- Solves real caching problems (especially for install tasks)
- Uses familiar gitignore-style syntax
- Is fully backward compatible
- Has minimal performance impact
- Provides clear migration and documentation path

The implementation is straightforward, leveraging the proven `vite_glob` crate, and integrates cleanly with existing fingerprint and cache systems.
