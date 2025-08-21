# Task Cache

Vite-plus implements a sophisticated caching system to avoid re-running tasks when their inputs haven't changed. This document describes the architecture, design decisions, and implementation details of the task cache system.

## Overview

The task cache system enables:

- **Incremental builds**: Only run tasks when inputs have changed
- **Shared caching**: Multiple tasks can share cache entries when appropriate
- **Content-based hashing**: Cache keys based on actual content, not timestamps
- **Output replay**: Cached stdout/stderr are replayed exactly as originally produced

### Shared caching

For tasks defined as below:

```jsonc
// package.json
{
  "scripts": {
    "build": "echo $foo",
    "test": "echo $foo && echo $bar"
  }
}
```

the task cache system is able to hit the same cache for `test` task and for the first subcommand in `build` task:

1. user runs `vite run build` -> no cache hit. run `echo $foo` and create cache
2. user runs `vite run test`
3. `echo $foo` -> **hit cache created in step 1 and replay**
4. `echo $bar` -> no cache hit. run `echo test` and create cache
5. user changes env `$foo`
6. user runs `vite run test`
7. `echo $foo`
   1. the cache system should be able to **locate the cache that was created in step 1 and hit in step 2.1**
   2. compare the command fingerprint and report cache miss because `$foo` is changed.
   3. re-run and replace the cache with a new one.
8. `echo $bar` -> hit cache created in step 2.2 and replay
9. user runs `vite run build`: **hit the cache created in step 4.1.3 and replay**.

## Architecture

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                     Task Execution Flow                                              │
├──────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  1. Task Request                                                                     │
│  ────────────────                                                                    │
│    app#build                                                                         │
│         │                                                                            │
│         ▼                                                                            │
│  2. Cache Key Generation                                                             │
│  ──────────────────────                                                              │
│    • Command fingerprint (includes cwd)                                              │
│    • Command fingerprint                                                             │
│    • Task arguments                                                                  │
│         │                                                                            │
│         ▼                                                                            │
│  3. Cache Lookup (SQLite)                                                            │
│  ────────────────────────                                                            │
│    ┌─────────────────┬──────────────────────┐──────────────────────────┐             │
│    │   Cache Hit     │   Cache Not Found    │  Cache Found but Miss    │             │
│    └────────┬────────┴─────────┬────────────┘──────────────────┬───────┘             │
│             │                  │                               │                     │
│             ▼                  ▼                               ▼                     │
│  4a. Validate Fingerprint   4b. Execute Task   ◀───── 4c. Report what change         |
│  ────────────────────────   ────────────────              causes the miss            │
│    • Config match?             • Run command                                         │
│    • Inputs unchanged?         • Monitor files (fspy)                                │
│    • Command same?             • Capture stdout/stderr                               │
│             │                         │                                              │
│             ▼                         ▼                                              │
│  5a. Replay Outputs        5b. Store in Cache                                        │
│  ──────────────────        ──────────────────                                        │
│    • Write to stdout           • Save fingerprint                                    │
│    • Write to stderr           • Save outputs                                        │
│                                • Update database                                     │
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

## Cache Key Components

### 1. Command Cache Key Structure

The command cache key uniquely identifies a command execution context:

```rust
pub struct CommandCacheKey {
    pub command_fingerprint: CommandFingerprint,    // Execution context
    pub args: Arc<[Str]>,                          // CLI arguments
}
```

The command fingerprint captures the complete execution context:

```rust
pub struct CommandFingerprint {
    pub cwd: Str,                                      // Working directory
    pub command: TaskCommand,                          // Shell script or command
    pub envs_without_pass_through: HashMap<Str, Str>,  // Environment variables
}

pub enum TaskCommand {
    Shell(Str),              // Raw shell script
    Parsed { bin: Str, args: Arc<[Str]> },  // Parsed command with args
}
```

This ensures cache invalidation when:

- Working directory changes
- Command or arguments change
- Environment variables differ

### 4. Command Fingerprinting

The complete task fingerprint includes input files:

```rust
pub struct CommandFingerprint {
    pub inputs: HashMap<Str, PathFingerprint>,      // Input file states
}
```

### 5. Task Cache Key Structure

The task id uniquely identifies a task:

```rust
pub struct TaskId {
    /// The name in `task.json`, or the name of the `package.json` script containing this task.
    /// See [`terminologies.md`](./terminologies.md) for details
    pub task_group_name: Str,

    /// the path of the package containing this task, relative to the monorepo root.
    /// We don't use package names as they can be the same for different packages.
    pub package_dir: Str,
    /// The index of the subcommand in a parsed command (`echo A && echo B`).
    /// None if the task is the last command.
    pub subcommand_index: Option<usize>,
}
```

### 6. (`CommandCacheKey`, `TaskId`) Relationship

The cache system maintains (`CommandCacheKey`, `TaskId`) relationship in order to locate the previous cache of the same task. This is a one-to-many relationship.

#### Input File Tracking

Vite-plus uses `fspy` to monitor file system access during task execution:

```
┌──────────────────────────────────────────────────────────────┐
│                  File System Monitoring                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Task Execution:                                             │
│  ──────────────                                              │
│    1. Start fspy monitoring                                  │
│    2. Execute task command                                   │
│    3. Capture accessed files                                 │
│    4. Stop monitoring                                        │
│         │                                                    │
│         ▼                                                    │
│  Fingerprint Generation:                                     │
│  ──────────────────────                                      │
│    For each accessed file:                                   │
│    • Check if file exists                                    │
│    • If file: Hash contents with xxHash3                     │
│    • If directory: Record structure                          │
│    • If missing: Mark as NotFound                            │
│         │                                                    │
│         ▼                                                    │
│  Path Fingerprint Types:                                     │
│  ──────────────────────                                      │
│    enum PathFingerprint {                                    │
│        NotFound,                   // File doesn't exist     │
│        FileContentHash(u64),       // xxHash3 of content     │
│        Folder(Option<HashMap>),    // Directory listing      │ 
|    }             ▲                                           │
│                  │                                           |  
|  This value is `None` when fspy reports that the task is     |  
|  opening a folder but not reading its entries. This can      |  
|  happen when the opened folder is used as a dirfd for        |  
|  `openat(2)`. In such case, the folder's entries don't need  |  
|  to be fingerprinted.                                        |  
|  Folders with empty entries fingerprinted are represented as |  
|  `Folder(Some(empty hashmap))`.                              |  
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 4. Fingerprint Validation

When a cache entry exists, the fingerprint is validated to detect changes:

```rust
pub enum CacheMiss {
    NotFound,                    // No cache entry exists
    FingerprintMismatch {        // Cache exists but invalid
        reason: FingerprintMismatchReason,
    },
}

pub enum FingerprintMismatchReason {
    ConfigChanged,               // Task configuration changed
    CommandChanged,              // Command fingerprint differs
    InputsChanged,               // Input files modified
}
```

## Cache Storage

### Storage Backend

Vite-plus uses SQLite with WAL (Write-Ahead Logging) mode for cache storage:

```rust
// Database initialization
let conn = Connection::open(cache_path)?;
conn.pragma_update(None, "journal_mode", "WAL")?;  // Better concurrency
conn.pragma_update(None, "synchronous", "NORMAL")?; // Balance speed/safety
```

### Database Schema

```sql
-- Simple key-value store for commands cache
CREATE TABLE commands (
    key BLOB PRIMARY KEY,    -- Serialized CommandsCacheKey
    value BLOB               -- Serialized CachedTask
);

-- One-to-many relationships between commands and tasks
CREATE TABLE commands_tasks (
    command_key BLOB,    -- Serialized CommandsCacheKey
    task_id BLOB           -- Serialized TaskId
);
```

### Serialization

Cache entries are serialized using `bincode` for efficient storage:

```rust
pub struct CachedTask {
    pub fingerprint: TaskFingerprint,      // Complete task state
    pub std_outputs: Arc<[StdOutput]>,     // Captured outputs
}

pub struct StdOutput {
    pub kind: OutputKind,                  // StdOut or StdErr
    pub content: MaybeString,              // Binary or UTF-8 content
}
```

## Cache Operations

### Cache Hit Flow

```
┌──────────────────────────────────────────────────────────────┐
│                      Cache Hit Process                       │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Generate Cache Key and task Id                           │
│  ─────────────────────                                       │
│    TaskCacheKey {                                            │
│        command_fingerprint: {...},                           │
│        args: ["--production"]                                │
│    }                                                         │
│    TaskId {                                                  │
│        task_group_name: "build",                             │
│        package_path: "./packages/app",                       │
|        subcommand_index: None,                               │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  2. Query SQLite Database                                    │
│  ────────────────────────                                    │
│    SELECT value FROM tasks WHERE key = ?                     │
│         │                                                    │
│         ▼                                                    │
│  3. Deserialize CachedTask                                   │
│  ─────────────────────────                                   │
│    CachedTask {                                              │
│        fingerprint: TaskFingerprint { ... },                 │
│        std_outputs: [StdOutput, ...]                         │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  4. Validate Fingerprint                                     │
│  ───────────────────────                                     │
│    • Compare resolved_config                                 │
│    • Check command_fingerprint                               │
│    • Verify input file hashes                                │
│         │                                                    │
│         ▼                                                    │
│  5. Replay Outputs                                           │
│  ─────────────────                                           │
│    For each StdOutput:                                       │
│    • Write to stdout/stderr                                  │
│    • Preserve original order                                 │
│    • Handle binary content                                   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Cache Miss and Storage

```
┌──────────────────────────────────────────────────────────────┐
│                    Cache Miss Process                        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Execute Task with Monitoring                             │
│  ───────────────────────────────                             │
│    • Start fspy file monitoring                              │
│    • Capture stdout/stderr                                   │
│    • Execute command                                         │
│    • Stop monitoring                                         │
│         │                                                    │
│         ▼                                                    │
│  2. Generate Fingerprint                                     │
│  ───────────────────────                                     │
│    • Hash all accessed files                                 │
│    • Record task configuration                               │
│    • Include command details                                 │
│         │                                                    │
│         ▼                                                    │
│  3. Create CachedTask                                        │
│  ────────────────────                                        │
│    CachedTask {                                              │
│        fingerprint: generated_fingerprint,                   │
│        std_outputs: captured_outputs                         │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  4. Store in Database                                        │
│  ────────────────────                                        │
│    INSERT OR REPLACE INTO tasks                              │
│    VALUES (serialized_key, serialized_value)                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Cache Invalidation

### Automatic Invalidation

Cache entries are automatically invalidated when:

1. **Command changes**: Different command, arguments, or working directory (includes package path)
2. **Package location changes**: Working directory in command fingerprint changes
3. **Environment changes**: Modified environment variables
4. **Input files change**: Content hash differs (detected via xxHash3)
5. **Configuration changes**: Task configuration in vite-task.json modified
6. **File structure changes**: Files added, removed, or type changed

### Fingerprint Mismatch Detection

```rust
// Fingerprint validation during cache lookup
fn validate_fingerprint(
    cached: &TaskFingerprint,
    current: &TaskFingerprint,
) -> Result<(), FingerprintMismatchReason> {
    // Check configuration
    if cached.resolved_config != current.resolved_config {
        return Err(FingerprintMismatchReason::ConfigChanged);
    }
    
    // Check command
    if cached.command_fingerprint != current.command_fingerprint {
        return Err(FingerprintMismatchReason::CommandChanged);
    }
    
    // Check input files
    for (path, fingerprint) in &current.inputs {
        if cached.inputs.get(path) != Some(fingerprint) {
            return Err(FingerprintMismatchReason::InputsChanged);
        }
    }
    
    Ok(())
}
```

## Performance Optimizations

### 1. Fast Hashing with xxHash3

Vite-plus uses xxHash3 for file content hashing, providing excellent performance:

```rust
use xxhash_rust::xxh3::xxh3_64;

pub fn hash_file_content(content: &[u8]) -> u64 {
    xxh3_64(content)  // ~10GB/s on modern CPUs
}
```

### 2. File System Monitoring

Instead of scanning all possible input files, `fspy` monitors actual file access:

```
┌──────────────────────────────────────────────────────────────┐
│              Efficient File Tracking                         │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Traditional Approach:                                       │
│  ────────────────────                                        │
│    Scan all src/**/*.ts files → Hash everything              │
│    Problem: Hashes files never accessed                      │
│                                                              │
│  Vite-plus Approach:                                         │
│  ──────────────────                                          │
│    Monitor with fspy → Hash only accessed files              │
│    Benefit: Minimal work, accurate dependencies              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 3. SQLite Optimizations

```rust
// WAL mode for better concurrency
conn.pragma_update(None, "journal_mode", "WAL")?;

// Balanced durability for performance
conn.pragma_update(None, "synchronous", "NORMAL")?;

// Prepared statements for efficiency
let mut stmt = conn.prepare_cached(
    "SELECT value FROM tasks WHERE key = ?"
)?;
```

### 4. Binary Serialization

Using `bincode` for compact, fast serialization:

```rust
// Efficient binary encoding
let key_bytes = bincode::encode_to_vec(&cache_key, config)?;
let value_bytes = bincode::encode_to_vec(&cached_task, config)?;

// Direct storage without text conversion
stmt.execute(params![key_bytes, value_bytes])?;
```

## Configuration

### Cache File Location

The cache database location can be configured via environment variable:

```bash
# Custom cache location
VITE_CACHE_PATH=/tmp/vite-cache.db vite-plus run build

# Default: .vite/cache.db in workspace root
vite-plus run build
```

### Task-Level Cache Control

Tasks can be marked as cacheable in `vite-task.json`:

```json
{
  "tasks": {
    "build": {
      "command": "tsc && rollup -c",
      "cacheable": true,
      "dependsOn": ["^build"]
    },
    "deploy": {
      "command": "deploy-script.sh",
      "cacheable": false // Never cache deployment tasks
    },
    "test": {
      "command": "jest",
      "cacheable": true
    }
  }
}
```

### Cache Behavior

- **Default**: Tasks are cacheable unless explicitly disabled
- **Compound commands**: Each subcommand cached independently
- **Dependencies**: Cache considers task dependencies

## Output Capture and Replay

### Output Capture During Execution

```rust
pub struct StdOutput {
    pub kind: OutputKind,        // StdOut or StdErr
    pub content: MaybeString,    // Binary-safe content
}

pub struct MaybeString(Vec<u8>);
```

Outputs are captured exactly as produced:

- Preserves order of stdout/stderr interleaving
- Handles binary output (e.g., from tools that output progress bars)
- Maintains ANSI color codes and formatting

### Output Replay on Cache Hit

When a task hits cache, outputs are replayed exactly:

```
┌──────────────────────────────────────────────────────────────┐
│                    Output Replay                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Cached Outputs:                                             │
│  ──────────────                                              │
│    [                                                         │
│      StdOutput { kind: StdOut, "Compiling..." },             │
│      StdOutput { kind: StdErr, "Warning: ..." },             │
│      StdOutput { kind: StdOut, "✓ Build complete" }          │
│    ]                                                         │
│         │                                                    │
│         ▼                                                    │
│  Replay Process:                                             │
│  ──────────────                                              │
│    1. Write "Compiling..." to stdout                         │
│    2. Write "Warning: ..." to stderr                         │
│    3. Write "✓ Build complete" to stdout                     │
│         │                                                    │
│         ▼                                                    │
│  Result: Identical output as original execution              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Implementation Examples

### Example: Cache Key for Named Package

```rust
// Task: app#build --production
TaskCacheKey {
    command_fingerprint: CommandFingerprint {
        cwd: "/monorepo/packages/app".into(),  // Package identified by cwd
        command: TaskCommand::Shell("tsc && rollup -c".into()),
        envs_without_pass_through: hashmap! {
            "NODE_ENV" => "production"
        },
    },
    args: vec!["--production"].into(),
}
```

### Example: Cache Key for Nameless Package

```rust
// Task in packages/frontend (no name in package.json)
TaskCacheKey {
    command_fingerprint: CommandFingerprint {
        cwd: "/monorepo/packages/frontend".into(),  // Package identified by cwd
        command: TaskCommand::Parsed {
            bin: "webpack".into(),
            args: vec!["--mode", "production"].into(),
        },
        envs_without_pass_through: HashMap::new(),
    },
    args: vec![].into(),
}
```

## Debugging Cache Behavior

### Environment Variables

```bash
# Enable debug logging
VITE_LOG=debug vite-plus run build

# Show cache operations
VITE_LOG=trace vite-plus run build
```

### Debug Output Examples

```
[DEBUG] Cache lookup for app#build
[DEBUG] Cache key: TaskCacheKey { command_fingerprint: ..., args: ... }
[DEBUG] Cache hit! Validating fingerprint...
[DEBUG] Fingerprint mismatch: InputsChanged
[DEBUG] File src/index.ts changed (hash: 0x1234... → 0x5678...)
[DEBUG] Cache miss, executing task
```

### Common Cache Miss Reasons

1. **ConfigChanged**: Task configuration in vite-task.json modified
2. **CommandChanged**: Command, args, or environment variables changed
3. **InputsChanged**: Source files modified or file structure changed
4. **NotFound**: No cache entry exists (first run or after cache clear)

## Best Practices

### 1. Deterministic Commands

Ensure commands produce identical outputs for identical inputs:

```json
// ❌ Bad: Non-deterministic output
{
  "tasks": {
    "build": {
      "command": "echo Built at $(date) && tsc"
    }
  }
}

// ✅ Good: Deterministic output
{
  "tasks": {
    "build": {
      "command": "tsc && echo Build complete"
    }
  }
}
```

### 2. Compound Commands for Efficiency

Leverage compound commands for granular caching:

```json
{
  "tasks": {
    "build": {
      // Each subcommand cached independently
      "command": "tsc && rollup -c && terser dist/bundle.js",
      "cacheable": true
    }
  }
}
```

Benefit: If only the final minification changes, TypeScript and bundling are served from cache.

### 3. Disable Cache for Side Effects

```json
{
  "tasks": {
    "deploy": {
      "command": "deploy-to-production.sh",
      "cacheable": false // Always run fresh
    },
    "notify": {
      "command": "slack-webhook.sh",
      "cacheable": false // Side effect: sends notification
    }
  }
}
```

### 4. File Access Patterns

The cache system automatically tracks accessed files:

```typescript
// This file access is automatically tracked
import config from './config.json';

// Dynamic imports are also tracked
const module = await import(`./locales/${lang}.json`);

// File system operations are monitored
const data = fs.readFileSync('data.txt');
```

No need to manually specify inputs - fspy captures actual dependencies.

## Implementation Reference

### Core Cache Components

```
┌──────────────────────────────────────────────────────────────┐
│                   Cache System Architecture                  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  crates/vite_task/src/                                       │
│  ├── cache.rs           # Cache storage and retrieval        │
│  │   ├── TaskCacheKey   # Cache key structure                │
│  │   ├── CachedTask     # Cached data structure              │
│  │   └── Cache          # Main cache interface               │
│  │                                                           │
│  ├── fingerprint.rs     # Fingerprint generation             │
│  │   ├── TaskFingerprint      # Complete task state          │
│  │   ├── PathFingerprint      # File/directory state         │
│  │   └── fingerprint_files()  # Hash file contents           │
│  │                                                           │
│  ├── execute.rs         # Task execution with caching        │
│  │   ├── execute_with_cache() # Main execution flow          │
│  │   ├── monitor_files()      # fspy integration             │
│  │   └── capture_outputs()    # Output collection            │
│  │                                                           │
│  └── schedule.rs        # Task scheduling and cache lookup   │
│      └── try_hit()      # Cache hit/miss detection           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Key Algorithms

#### Cache Key Generation

```rust
// Simplified from actual implementation
impl TaskCacheKey {
    pub fn new(resolved: &ResolvedTask) -> Self {
        Self {
            command_fingerprint: resolved.command_fingerprint.clone(),
            args: resolved.args.clone(),
        }
    }
}
```

#### Fingerprint Validation

```rust
// Validates cached fingerprint against current state
pub fn validate(
    cached: &TaskFingerprint,
    current: &TaskFingerprint,
) -> Result<(), FingerprintMismatchReason> {
    // Compare all components
    if cached.resolved_config != current.resolved_config {
        return Err(FingerprintMismatchReason::ConfigChanged);
    }
    if cached.command_fingerprint != current.command_fingerprint {
        return Err(FingerprintMismatchReason::CommandChanged);
    }
    if cached.inputs != current.inputs {
        return Err(FingerprintMismatchReason::InputsChanged);
    }
    Ok(())
}
```

### Performance Characteristics

- **Cache key generation**: ~1μs per task
- **File hashing**: ~10GB/s with xxHash3
- **Database operations**: <1ms for typical queries
- **Fingerprint validation**: ~10μs per task
- **Output replay**: Near-zero overhead

The cache system adds minimal overhead while providing significant speedups for unchanged tasks, making incremental builds in large monorepos extremely efficient.
