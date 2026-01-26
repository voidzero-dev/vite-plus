# RFC: JavaScript Runtime Management (`vite_js_runtime`)

## Background

Currently, vite-plus relies on the user's system-installed Node.js runtime. This creates several challenges:

1. **Version inconsistency**: Different team members may have different Node.js versions installed, leading to subtle bugs and "works on my machine" issues
2. **CI/CD complexity**: Build pipelines need explicit Node.js version management
3. **No runtime pinning**: Projects cannot specify and enforce a specific Node.js version
4. **Future extensibility**: As alternatives like Bun and Deno mature, projects may want to use different runtimes

The PackageManager implementation in `vite_install` successfully handles automatic downloading and caching of package managers (pnpm, yarn, npm). We can apply the same pattern to JavaScript runtimes.

## Goals

1. **Pure library design**: A library crate that receives runtime name and version as input, downloads and caches the runtime, and returns the installation path
2. **Cross-platform support**: Handle Windows, macOS, and Linux with appropriate binaries
3. **Consistent caching**: Use the same global cache directory pattern as PackageManager
4. **Extensible design**: Support Node.js initially, with architecture ready for Bun and Deno

## Non-Goals (Initial Version)

- Configuration auto-detection (no reading from package.json, .nvmrc, etc.)
- Managing multiple runtime versions simultaneously
- Providing a version manager CLI (like nvm/fnm)
- Supporting custom/unofficial Node.js builds

## Input Format

The library accepts runtime specification as a string parameter:

```
<runtime>@<version>
```

### Examples

| Runtime | Example |
|---|---|
| Node.js | `node@22.13.1` |
| Bun (future) | `bun@1.2.0` |
| Deno (future) | `deno@2.0.0` |

Only exact versions are supported. Version aliases (like `latest` or `lts`) may be added in future versions.

## Architecture

### Crate Structure

```
crates/vite_js_runtime/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API exports
    ├── runtime.rs          # JsRuntime struct and core logic
    ├── node.rs             # Node.js specific implementation
    ├── download.rs         # Download and extraction utilities
    └── platform.rs         # Platform detection and binary selection
```

### Core Types

```rust
/// Supported JavaScript runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsRuntimeType {
    Node,
    // Future: Bun, Deno
}

/// Represents a downloaded JavaScript runtime
pub struct JsRuntime {
    pub runtime_type: JsRuntimeType,
    pub version: Str,                   // Resolved version (e.g., "22.13.1")
    pub install_dir: AbsolutePathBuf,
}
```

### Public API

```rust
/// Parse a runtime specification string (e.g., "node@22.13.1")
pub fn parse_runtime_spec(spec: &str) -> Result<(JsRuntimeType, String), Error>;

/// Download and cache a JavaScript runtime
/// Returns the JsRuntime with installation path
pub async fn download_runtime(
    runtime_type: JsRuntimeType,
    version: &str,           // Exact version (e.g., "22.13.1")
) -> Result<JsRuntime, Error>;

impl JsRuntime {
    /// Get the path to the runtime binary (e.g., node, bun)
    pub fn get_binary_path(&self) -> AbsolutePathBuf;

    /// Get the bin directory containing the runtime
    pub fn get_bin_prefix(&self) -> AbsolutePathBuf;

    /// Get the runtime type
    pub fn runtime_type(&self) -> JsRuntimeType;

    /// Get the resolved version string (always exact, e.g., "22.13.1")
    pub fn version(&self) -> &str;
}
```

### Usage Example

```rust
use vite_js_runtime::{JsRuntimeType, download_runtime, parse_runtime_spec};

// Option 1: Direct download with known runtime type
let runtime = download_runtime(JsRuntimeType::Node, "22.13.1").await?;
println!("Node.js installed at: {}", runtime.get_binary_path());

// Option 2: Parse spec string first
let (runtime_type, version) = parse_runtime_spec("node@22.13.1")?;
let runtime = download_runtime(runtime_type, &version).await?;
println!("Version: {}", runtime.version()); // "22.13.1"
```

## Cache Directory Structure

Following the PackageManager pattern:

```
$CACHE_DIR/vite/js_runtime/{runtime}/{version}/{platform}-{arch}/
```

Examples:
- Linux x64: `~/.cache/vite/js_runtime/node/22.13.1/linux-x64/`
- macOS ARM: `~/Library/Caches/vite/js_runtime/node/22.13.1/darwin-arm64/`
- Windows x64: `%LOCALAPPDATA%\vite\js_runtime\node\22.13.1\win-x64\`

### Platform Detection

| OS | Architecture | Platform String |
|---|---|---|
| Linux | x64 | `linux-x64` |
| Linux | ARM64 | `linux-arm64` |
| macOS | x64 | `darwin-x64` |
| macOS | ARM64 | `darwin-arm64` |
| Windows | x64 | `win-x64` |
| Windows | ARM64 | `win-arm64` |

## Download Sources

### Node.js

Official distribution from nodejs.org:

```
https://nodejs.org/dist/v{version}/node-v{version}-{platform}.{ext}
```

| Platform | Archive Format | Example |
|---|---|---|
| Linux | `.tar.gz` | `node-v22.13.1-linux-x64.tar.gz` |
| macOS | `.tar.gz` | `node-v22.13.1-darwin-arm64.tar.gz` |
| Windows | `.zip` | `node-v22.13.1-win-x64.zip` |

### Integrity Verification

Node.js provides SHASUMS256.txt for each release:
```
https://nodejs.org/dist/v{version}/SHASUMS256.txt
```

The implementation verifies download integrity automatically:
1. Download SHASUMS256.txt for the target version
2. Parse and extract the SHA256 hash for the target archive filename
3. After downloading the archive, verify it against the expected hash
4. Fail with error if hash doesn't match (corrupted download)

Example SHASUMS256.txt content:
```
a1b2c3d4...  node-v22.13.1-darwin-arm64.tar.gz
e5f6g7h8...  node-v22.13.1-darwin-x64.tar.gz
i9j0k1l2...  node-v22.13.1-linux-arm64.tar.gz
...
```

## Implementation Details

### Download Flow

```
1. Receive runtime type and exact version as input

2. Determine platform and architecture
   └── Map to Node.js distribution naming

3. Check cache for existing installation
   └── If exists: return cached path
   └── If not: continue to download

4. Download with atomic operations
   ├── Create temp directory
   ├── Download SHASUMS256.txt and parse expected hash
   ├── Download archive with retry logic
   ├── Verify archive hash against SHASUMS256.txt
   ├── Extract archive
   ├── Acquire file lock (prevent concurrent installs)
   └── Atomic rename to final location

5. Return JsRuntime with install path
```

### Concurrent Download Protection

Same pattern as PackageManager:
- Use tempfile for atomic operations
- File-based locking to prevent race conditions
- Check cache after acquiring lock (another process may have completed)

## Integration with vite_install

The `vite_install` crate can use `vite_js_runtime` to:
1. Ensure the correct Node.js version before running package manager commands
2. Use the managed Node.js to execute package manager binaries

```rust
// Example integration in vite_install
use vite_js_runtime::{JsRuntimeType, download_runtime};

async fn run_with_managed_node(
    node_version: &str,
    args: &[&str],
) -> Result<(), Error> {
    // Download/cache the runtime
    let runtime = download_runtime(JsRuntimeType::Node, node_version).await?;

    // Use the managed Node.js binary
    let node_path = runtime.get_binary_path();

    // Execute command with managed Node.js
    Command::new(node_path)
        .args(args)
        .spawn()?
        .wait()?;

    Ok(())
}
```

## Error Handling

New error variants for `vite_error`:

```rust
pub enum JsRuntimeError {
    /// Invalid runtime specification format
    InvalidRuntimeSpec { spec: String },

    /// Unsupported runtime type
    UnsupportedRuntime { runtime: String },

    /// Version not found in official releases
    VersionNotFound { runtime: String, version: String },

    /// Platform not supported for this runtime
    UnsupportedPlatform { platform: String, runtime: String },

    /// Download failed after retries
    DownloadFailed { url: String, reason: String },

    /// Hash verification failed (download corrupted)
    HashMismatch { expected: String, actual: String },

    /// Archive extraction failed
    ExtractionFailed { reason: String },
}
```

## Testing Strategy

### Unit Tests

1. **Runtime spec parsing**
   - Valid formats: `node@22.13.1`
   - Invalid formats: `node`, `22.13.1`, `unknown@1.0.0`, `node@`

2. **Platform detection**
   - Test all supported platform/arch combinations
   - Test mapping to Node.js distribution names

3. **Cache path generation**
   - Verify correct directory structure

### Integration Tests

1. **Download and cache**
   - Download a specific Node.js version
   - Verify binary exists and is executable
   - Verify cache reuse on second call

2. **Integrity verification**
   - Test successful verification against SHASUMS256.txt
   - Test failure when archive is corrupted (hash mismatch)

3. **Concurrent downloads**
   - Simulate multiple processes downloading same version
   - Verify no corruption or conflicts

## Design Decisions

### 1. Pure Library vs. Configuration-Aware

**Decision**: Pure library that receives runtime name and version as input.

**Rationale**:
- Maximum flexibility - callers decide how to obtain the runtime specification
- No coupling to specific configuration formats (package.json, .nvmrc, etc.)
- Easier to test in isolation
- Clear single responsibility: download and cache runtimes

### 2. Separate Crate vs. Extending vite_install

**Decision**: Create a new `vite_js_runtime` crate.

**Rationale**:
- Clear separation of concerns (runtime vs. package manager)
- Reusable by other crates without pulling in package manager logic
- Easier to maintain and test independently
- Follows existing crate organization pattern

### 3. Version Specification Format

**Decision**: Use `runtime@version` format with exact versions only.

**Rationale**:
- Mirrors the established `packageManager` format
- Exact versions ensure reproducibility
- No network requests needed for version resolution
- Simpler implementation without caching complexity
- Version aliases can be added as a future enhancement

### 4. Initial Node.js Only

**Decision**: Support only Node.js in the initial version.

**Rationale**:
- Node.js is the most widely used runtime
- Allows focused, well-tested implementation
- Architecture supports easy addition of Bun/Deno later
- Reduces initial complexity and scope

## Future Enhancements

1. **Version aliases**: Support `latest` and `lts` aliases with cached version index
2. **Bun support**: Add `bun@x.y.z` runtime option with Bun release downloads
3. **Deno support**: Add `deno@x.y.z` runtime option with Deno release downloads
4. **Version ranges**: Support semver ranges like `node@^22.0.0`
5. **Custom mirrors**: Support custom download URLs for corporate environments
6. **Offline mode**: Use cached versions without network access

## Success Criteria

1. ✅ Can download and cache Node.js by exact version specification
2. ✅ Works on Linux, macOS, and Windows (x64 and ARM64)
3. ✅ Verifies download integrity using SHASUMS256.txt
4. ✅ Handles concurrent downloads safely
5. ✅ Returns version and binary path
6. ✅ Comprehensive test coverage

## References

- [Node.js Releases](https://nodejs.org/en/download/releases/)
- [Node.js Distribution Index](https://nodejs.org/dist/index.json)
- [Corepack (Node.js Package Manager Manager)](https://nodejs.org/api/corepack.html)
- [fnm (Fast Node Manager)](https://github.com/Schniz/fnm)
- [volta (JavaScript Tool Manager)](https://volta.sh/)
