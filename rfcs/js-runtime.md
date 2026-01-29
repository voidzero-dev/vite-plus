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

- ~~Configuration auto-detection (no reading from package.json, .nvmrc, etc.)~~ **Now supported via `devEngines.runtime`**
- Managing multiple runtime versions simultaneously
- Providing a version manager CLI (like nvm/fnm)
- Supporting custom/unofficial Node.js builds

## Input Format

The library accepts runtime specification as a string parameter:

```
<runtime>@<version>
```

### Examples

| Runtime       | Example        |
| ------------- | -------------- |
| Node.js       | `node@22.13.1` |
| Bun (future)  | `bun@1.2.0`    |
| Deno (future) | `deno@2.0.0`   |

Both exact versions and semver ranges are supported:

- Exact: `22.13.1`
- Caret range: `^22.0.0` (>=22.0.0 <23.0.0)
- Tilde range: `~22.13.0` (>=22.13.0 <22.14.0)
- Latest: omit version to get the latest release

## Architecture

### Crate Structure

```
crates/vite_js_runtime/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API exports
    ├── dev_engines.rs      # devEngines.runtime parsing from package.json
    ├── error.rs            # Error types
    ├── platform.rs         # Platform detection (Os, Arch, Platform)
    ├── provider.rs         # JsRuntimeProvider trait and types
    ├── providers/          # Provider implementations
    │   ├── mod.rs
    │   └── node.rs         # NodeProvider with version resolution
    ├── download.rs         # Generic download utilities
    └── runtime.rs          # JsRuntime struct and download orchestration
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
    binary_relative_path: Str,          // e.g., "bin/node" or "node.exe"
    bin_dir_relative_path: Str,         // e.g., "bin" or ""
}

/// Archive format for runtime distributions
pub enum ArchiveFormat {
    TarGz,  // .tar.gz (Linux, macOS)
    Zip,    // .zip (Windows)
}

/// How to verify the integrity of a downloaded archive
pub enum HashVerification {
    ShasumsFile { url: Str },  // Download and parse SHASUMS file
    None,                       // No verification
}

/// Information needed to download a runtime
pub struct DownloadInfo {
    pub archive_url: Str,
    pub archive_filename: Str,
    pub archive_format: ArchiveFormat,
    pub hash_verification: HashVerification,
    pub extracted_dir_name: Str,
}
```

### Provider Trait

The `JsRuntimeProvider` trait abstracts runtime-specific logic, making it easy to add new runtimes:

```rust
#[async_trait]
pub trait JsRuntimeProvider: Send + Sync {
    /// Get the name of this runtime (e.g., "node", "bun", "deno")
    fn name(&self) -> &'static str;

    /// Get the platform string used in download URLs
    fn platform_string(&self, platform: Platform) -> Str;

    /// Get download information for a specific version and platform
    fn get_download_info(&self, version: &str, platform: Platform) -> DownloadInfo;

    /// Get the relative path to the runtime binary from the install directory
    fn binary_relative_path(&self, platform: Platform) -> Str;

    /// Get the relative path to the bin directory from the install directory
    fn bin_dir_relative_path(&self, platform: Platform) -> Str;

    /// Parse a SHASUMS file to extract the hash for a specific filename
    fn parse_shasums(&self, shasums_content: &str, filename: &str) -> Result<Str, Error>;
}
```

### Adding a New Runtime

To add support for a new runtime (e.g., Bun):

1. Create `src/providers/bun.rs` implementing `JsRuntimeProvider`
2. Add `Bun` variant to `JsRuntimeType` enum
3. Add match arm in `download_runtime()` to use the new provider
4. Export the provider from `src/providers/mod.rs`

### Public API

```rust
/// Download and cache a JavaScript runtime by exact version
pub async fn download_runtime(
    runtime_type: JsRuntimeType,
    version: &str,           // Exact version (e.g., "22.13.1")
) -> Result<JsRuntime, Error>;

/// Download runtime based on project's devEngines.runtime configuration
/// Reads package.json, resolves semver ranges, downloads the matching version
/// If no version was specified, writes the resolved version back to package.json
pub async fn download_runtime_for_project(
    project_path: &AbsolutePath,
) -> Result<JsRuntime, Error>;

/// Update devEngines.runtime in package.json with the resolved version
/// Preserves original formatting (indentation, key order, trailing newline)
pub async fn update_runtime_version(
    package_json_path: &AbsolutePath,
    runtime_name: &str,
    version: &str,
) -> Result<(), Error>;

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

impl NodeProvider {
    /// Fetch version index from nodejs.org/dist/index.json (with HTTP caching)
    pub async fn fetch_version_index(&self) -> Result<Vec<NodeVersionEntry>, Error>;

    /// Resolve version requirement (e.g., "^24.4.0") to exact version
    pub async fn resolve_version(&self, version_req: &str) -> Result<Str, Error>;

    /// Get latest version (first entry in index)
    pub async fn resolve_latest_version(&self) -> Result<Str, Error>;
}
```

### Usage Examples

**Direct version download:**

```rust
use vite_js_runtime::{JsRuntimeType, download_runtime};

let runtime = download_runtime(JsRuntimeType::Node, "22.13.1").await?;
println!("Node.js installed at: {}", runtime.get_binary_path());
println!("Version: {}", runtime.version()); // "22.13.1"
```

**Project-based download (reads devEngines.runtime from package.json):**

```rust
use vite_js_runtime::download_runtime_for_project;
use vite_path::AbsolutePathBuf;

let project_path = AbsolutePathBuf::new("/path/to/project".into()).unwrap();
let runtime = download_runtime_for_project(&project_path).await?;
// Version is resolved from devEngines.runtime or uses latest
```

## Cache Directory Structure

Following the PackageManager pattern:

```
$CACHE_DIR/vite/js_runtime/{runtime}/{version}/
```

Examples:

- Linux x64: `~/.cache/vite/js_runtime/node/22.13.1/`
- macOS ARM: `~/Library/Caches/vite/js_runtime/node/22.13.1/`
- Windows x64: `%LOCALAPPDATA%\vite\js_runtime\node\22.13.1\`

### Version Index Cache

The Node.js version index is cached locally to avoid repeated network requests:

```
$CACHE_DIR/vite/js_runtime/node/index_cache.json
```

Cache structure:

```json
{
  "expires_at": 1706400000,
  "etag": null,
  "versions": [
    {"version": "v25.5.0", "lts": false},
    {"version": "v24.4.0", "lts": "Jod"},
    ...
  ]
}
```

- Default TTL: 1 hour (3600 seconds)
- Cache is refreshed when expired
- Falls back to full fetch if cache is corrupted

### Platform Detection

| OS      | Architecture | Platform String |
| ------- | ------------ | --------------- |
| Linux   | x64          | `linux-x64`     |
| Linux   | ARM64        | `linux-arm64`   |
| macOS   | x64          | `darwin-x64`    |
| macOS   | ARM64        | `darwin-arm64`  |
| Windows | x64          | `win-x64`       |
| Windows | ARM64        | `win-arm64`     |

## Project Configuration (devEngines.runtime)

The `download_runtime_for_project` function reads the `devEngines.runtime` field from the project's package.json. This follows the [npm devEngines RFC](https://github.com/npm/rfcs/blob/main/accepted/0048-devEngines.md).

### Single Runtime

```json
{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^24.4.0",
      "onFail": "download"
    }
  }
}
```

### Multiple Runtimes (Array)

```json
{
  "devEngines": {
    "runtime": [
      {
        "name": "node",
        "version": "^24.4.0",
        "onFail": "download"
      },
      {
        "name": "deno",
        "version": "^2.4.3",
        "onFail": "download"
      }
    ]
  }
}
```

**Note:** Currently only the `"node"` runtime is supported. Other runtimes are ignored.

### Version Resolution

The version resolution is optimized to minimize network requests:

| Version Specified  | Local Cache | Network Request | Result                     |
| ------------------ | ----------- | --------------- | -------------------------- |
| Exact (`20.18.0`)  | -           | **No**          | Use exact version directly |
| Range (`^20.18.0`) | Match found | **No**          | Use cached version         |
| Range (`^20.18.0`) | No match    | **Yes**         | Resolve from network       |
| Empty/None         | -           | **Yes**         | Get latest LTS version     |

**Exact versions** (e.g., `20.18.0`, `v20.18.0`) are detected using `node_semver::Version::parse()` and used directly without network validation. The `v` prefix is normalized (stripped) since download URLs already add it.

**Partial versions** like `20` or `20.18` are treated as ranges, not exact versions.

**Semver ranges** (e.g., `^24.4.0`) trigger version resolution:

1. First, check locally cached Node.js installations for a version that satisfies the range
2. If a matching cached version exists, use the highest one (no network request)
3. Otherwise, fetch the version index from `https://nodejs.org/dist/index.json`
4. Cache the index locally with 1-hour TTL (supports ETag-based conditional requests)
5. Use `node-semver` crate for npm-compatible range matching
6. Return the highest version that satisfies the range

### Fallback Behavior

- If no `devEngines.runtime` is configured, downloads the latest Node.js version
- If `name` is not `"node"`, the runtime is skipped
- If `version` is empty, downloads the latest Node.js version

### Version Write-Back

When `download_runtime_for_project` resolves a version (i.e., no version was specified), it writes the resolved version back to `package.json`. This ensures subsequent executions can skip version resolution and use the cached exact version directly.

**Write-back occurs when:**

- `devEngines.runtime` doesn't exist (creates the entire structure)
- `devEngines.runtime` exists but has no `version` field
- `devEngines.runtime` is an array and the matching entry has no `version` field

**Write-back does NOT occur when:**

- A version range is already specified (e.g., `^20.18.0`)
- An exact version is already specified (e.g., `20.18.0`)

**Example: Before download (no version specified)**

```json
{
  "name": "my-project",
  "devEngines": {
    "runtime": {
      "name": "node"
    }
  }
}
```

**After download (version written back)**

```json
{
  "name": "my-project",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "24.5.0"
    }
  }
}
```

**Formatting preservation:**

- Original indentation style is preserved (2 spaces, 4 spaces, or tabs)
- Key order is preserved using `serde_json` with `preserve_order` feature
- Trailing newline is preserved if present in original

## Download Sources

### Node.js

Official distribution from nodejs.org:

```
https://nodejs.org/dist/v{version}/node-v{version}-{platform}.{ext}
```

| Platform | Archive Format | Example                             |
| -------- | -------------- | ----------------------------------- |
| Linux    | `.tar.gz`      | `node-v22.13.1-linux-x64.tar.gz`    |
| macOS    | `.tar.gz`      | `node-v22.13.1-darwin-arm64.tar.gz` |
| Windows  | `.zip`         | `node-v22.13.1-win-x64.zip`         |

### Custom Mirror Support

The distribution URL can be overridden using the `VITE_NODE_DIST_MIRROR` environment variable. This is useful for corporate environments or regions where nodejs.org might be slow or blocked.

```bash
VITE_NODE_DIST_MIRROR=https://example.com/mirrors/node vite build
```

The mirror URL should have the same directory structure as the official distribution. Trailing slashes are automatically trimmed.

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

2. Select the appropriate JsRuntimeProvider
   └── e.g., NodeProvider for JsRuntimeType::Node

3. Get download info from provider
   ├── Platform string (e.g., "linux-x64", "win-x64")
   ├── Archive URL and filename
   ├── Hash verification method
   └── Extracted directory name

4. Check cache for existing installation
   └── If exists: return cached path
   └── If not: continue to download

5. Download with atomic operations
   ├── Create temp directory
   ├── Download SHASUMS file and parse expected hash (via provider)
   ├── Download archive with retry logic
   ├── Verify archive hash
   ├── Extract archive (tar.gz or zip based on format)
   ├── Acquire file lock (prevent concurrent installs)
   └── Atomic rename to final location

6. Return JsRuntime with install path and relative paths
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

Error variants in `vite_js_runtime::Error`:

```rust
pub enum Error {
    /// Version not found in official releases
    VersionNotFound { runtime: Str, version: Str },

    /// Platform not supported for this runtime
    UnsupportedPlatform { platform: Str, runtime: Str },

    /// Download failed after retries
    DownloadFailed { url: Str, reason: Str },

    /// Hash verification failed (download corrupted)
    HashMismatch { filename: Str, expected: Str, actual: Str },

    /// Archive extraction failed
    ExtractionFailed { reason: Str },

    /// SHASUMS file parsing failed
    ShasumsParseFailed { reason: Str },

    /// Hash not found in SHASUMS file
    HashNotFound { filename: Str },

    /// Failed to parse version index
    VersionIndexParseFailed { reason: Str },

    /// No version matching the requirement found
    NoMatchingVersion { version_req: Str },

    /// IO, HTTP, JSON, and semver errors
    Io(std::io::Error),
    Reqwest(reqwest::Error),
    JoinError(tokio::task::JoinError),
    Json(serde_json::Error),
    SemverRange(node_semver::SemverError),
}
```

## Testing Strategy

### Unit Tests

1. **Platform detection**
   - Test all supported platform/arch combinations
   - Test mapping to Node.js distribution names

2. **Cache path generation**
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

**Decision**: Support both exact versions and semver ranges.

**Rationale**:

- Mirrors the established `packageManager` format for exact versions
- Semver ranges provide flexibility for automatic updates within constraints
- Version index is cached locally (1-hour TTL) to minimize network requests
- Uses `node-semver` crate for npm-compatible range parsing
- `download_runtime()` takes exact versions; `download_runtime_for_project()` handles range resolution

### 4. Initial Node.js Only

**Decision**: Support only Node.js in the initial version.

**Rationale**:

- Node.js is the most widely used runtime
- Allows focused, well-tested implementation
- Trait-based architecture (`JsRuntimeProvider`) makes adding Bun/Deno straightforward
- Reduces initial complexity and scope

### 5. Trait-Based Provider Architecture

**Decision**: Use a `JsRuntimeProvider` trait to abstract runtime-specific logic.

**Rationale**:

- Clean separation between generic download logic and runtime-specific details
- Each provider encapsulates: platform strings, URL construction, hash verification, binary paths
- Adding a new runtime only requires implementing the trait
- Generic download utilities are reusable across all providers

## Future Enhancements

1. ✅ **Version aliases**: Support `latest` alias with cached version index
2. **Bun support**: Create `BunProvider` implementing `JsRuntimeProvider`
3. **Deno support**: Create `DenoProvider` implementing `JsRuntimeProvider`
4. ✅ **Version ranges**: Support semver ranges like `node@^22.0.0`
5. **Offline mode**: Full offline support (partial: ranges check local cache first)
6. **LTS alias**: Support `lts` alias to download latest LTS version

## Success Criteria

1. ✅ Can download and cache Node.js by exact version specification
2. ✅ Works on Linux, macOS, and Windows (x64 and ARM64)
3. ✅ Verifies download integrity using SHASUMS256.txt
4. ✅ Handles concurrent downloads safely
5. ✅ Returns version and binary path
6. ✅ Comprehensive test coverage
7. ✅ Custom mirrors via `VITE_NODE_DIST_MIRROR` environment variable
8. ✅ Support `devEngines.runtime` from package.json
9. ✅ Support semver ranges (^, ~, etc.) with version resolution
10. ✅ Version index caching with 1-hour TTL
11. ✅ Support both single runtime and array of runtimes in devEngines
12. ✅ Write resolved version back to package.json (with formatting preservation)
13. ✅ Optimized version resolution (skip network for exact versions, check local cache for ranges)

## References

- [Node.js Releases](https://nodejs.org/en/download/releases/)
- [Node.js Distribution Index](https://nodejs.org/dist/index.json)
- [Corepack (Node.js Package Manager Manager)](https://nodejs.org/api/corepack.html)
- [fnm (Fast Node Manager)](https://github.com/Schniz/fnm)
- [volta (JavaScript Tool Manager)](https://volta.sh/)
