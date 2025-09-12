# Task Cache

Vite-plus implements a sophisticated caching system to avoid re-running tasks when their inputs haven't changed. This document describes the architecture, design decisions, and implementation details of the task cache system.

## Overview

The task cache system enables:

- **Incremental builds**: Only run tasks when inputs have changed
- **Shared caching**: Multiple tasks with identical commands can share cache entries
- **Individual task run caching**: Tasks with different arguments get separate cache entries
- **Content-based hashing**: Cache keys based on actual content, not timestamps
- **Output replay**: Cached stdout/stderr are replayed exactly as originally produced
- **Two-tier caching**: Command-level cache shared across tasks, with task-run associations

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
   1. `echo $foo` -> **hit cache created in step 1 and replay**
   2. `echo $bar` -> no cache hit. run `echo test` and create cache
3. user changes env `$foo`
4. user runs `vite run test`
   1. `echo $foo`
      1. the cache system should be able to **locate the cache that was created in step 1 and hit in step 2.1**
      2. compare the command fingerprint and report cache miss because `$foo` is changed.
      3. re-run and replace the cache with a new one.
   2. `echo $bar` -> hit cache created in step 2.2 and replay
5. user runs `vite run build`: **hit the cache created in step 4.1.3 and replay**.

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
│    • Task arguments                                                                  │
│    • Environment variables                                                           │
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
    pub cwd: RelativePathBuf,                          // Working directory, relative to workspace root
    pub command: TaskCommand,                          // Shell script or parsed command
    pub envs_without_pass_through: HashMap<Str, Str>,  // Environment variables (excludes pass-through)
}

pub enum TaskCommand {
    ShellScript(Str),                    // Raw shell script
    Parsed(TaskParsedCommand),           // Parsed command with program and args
}
```

This ensures cache invalidation when:

- Working directory changes (package location changes)
- Command or arguments change
- Declared environment variables differ (pass-through envs don't affect cache)

### 2. Environment Variable Impact on Cache

The `envs_without_pass_through` field is crucial for cache correctness:

- Only includes envs explicitly declared in the task's `envs` array
- Does NOT include pass-through envs (PATH, CI, etc.)
- These envs become part of the cache key

When a task runs:

1. All envs (including pass-through) are available to the process
2. Only declared envs affect the cache key
3. If a declared env changes value, cache will miss
4. If a pass-through env changes, cache will still hit

For built-in tasks (lint, build, test):

- The resolver provides envs which become part of the fingerprint
- If resolver provides different envs between runs, cache breaks
- Each built-in task type must have unique task name to avoid cache collision

### 3. Task Fingerprinting

The complete task fingerprint includes input files tracked during execution:

```rust
pub struct TaskFingerprint {
    pub resolved_config: ResolvedTaskConfig,        // Task configuration
    pub command_fingerprint: CommandFingerprint,    // Command execution context
    pub inputs: HashMap<RelativePathBuf, PathFingerprint>,  // Input file states
}
```

### 4. Task ID Structure

The task ID uniquely identifies a task:

```rust
pub struct TaskId {
    /// The name in `vite-task.json`, or the name of the `package.json` script containing this task.
    /// See [`terminologies.md`](./terminologies.md) for details
    pub task_group_name: Str,

    /// The path of the package containing this task, relative to the monorepo root.
    /// We don't use package names as they can be the same for different packages.
    pub package_dir: RelativePathBuf,
    
    /// The index of the subcommand in a parsed command (`echo A && echo B`).
    /// None if the task is the last command.
    pub subcommand_index: Option<usize>,
}
```

### 5. (`CommandCacheKey`, `TaskId`) Relationship

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

### 6. Fingerprint Validation

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
│  1. Generate Cache Keys                                      │
│  ──────────────────────                                      │
│    TaskRunKey {                                              │
│        task_id: TaskId { ... },                              │
│        args: ["--production"]                                │
│    }                                                         │
│    CommandFingerprint {                                      │
│        cwd: "packages/app",                                  │
│        command: Parsed(...),                                 │
│        envs_without_pass_through: {...},                     │
│        pass_through_envs: {...}                              │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  2. Query Command Cache                                      │
│  ──────────────────────                                      │
│    SELECT value FROM command_cache WHERE key = command_fp    │
│         │                                                    │
│         ▼                                                    │
│  3. Deserialize CommandCacheValue                            │
│  ─────────────────────────────                               │
│    CommandCacheValue {                                       │
│        post_run_fingerprint: PostRunFingerprint { ... },     │
│        std_outputs: [StdOutput, ...]                         │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  4. Validate Post-Run Fingerprint                           │
│  ─────────────────────────────────                           │
│    • Check input file hashes                                │
│    • Detect file content changes                            │
│         │                                                    │
│         ▼                                                    │
│  5. Replay Outputs & Update Association                     │
│  ──────────────────────────────────────                     │
│    • Write to stdout/stderr                                  │
│    • Preserve original order                                 │
│    • Update taskrun_to_command mapping                       │
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
│  2. Generate Post-Run Fingerprint                           │
│  ─────────────────────────────────                           │
│    • Hash all accessed files                                 │
│    • Record file system access patterns                     │
│         │                                                    │
│         ▼                                                    │
│  3. Create CommandCacheValue                                 │
│  ──────────────────────────                                  │
│    CommandCacheValue {                                       │
│        post_run_fingerprint: generated_fingerprint,          │
│        std_outputs: captured_outputs                         │
│    }                                                         │
│         │                                                    │
│         ▼                                                    │
│  4. Store in Database Tables                                 │
│  ───────────────────────────                                 │
│    INSERT OR REPLACE INTO command_cache                      │
│    VALUES (command_fingerprint, cache_value)                 │
│    INSERT OR REPLACE INTO taskrun_to_command                 │
│    VALUES (task_run_key, command_fingerprint)                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Cache Invalidation

### Automatic Invalidation

Cache entries are automatically invalidated when:

1. **Command changes**: Different command, arguments, or working directory
2. **Package location changes**: Working directory (`cwd`) in command fingerprint changes
3. **Environment changes**: Modified declared environment variables (pass-through values don't affect cache)
4. **Pass-through config changes**: Pass-through environment names added/removed from configuration
5. **Input files change**: Content hash differs (detected via xxHash3)
6. **File structure changes**: Files added, removed, or type changed
7. **Built-in task location**: Built-in tasks run from different directories get separate caches

### Fingerprint Mismatch Detection

```rust
// Two-level fingerprint validation during cache lookup
pub async fn try_hit(
    &self,
    task: &ResolvedTask,
    fs: &impl FileSystem,
    base_dir: &AbsolutePath,
) -> Result<Result<CommandCacheValue, CacheMiss>, Error> {
    let task_run_key = TaskRunKey { task_id: task.id(), args: task.args.clone() };
    let command_fingerprint = &task.resolved_command.fingerprint;
    
    if let Some(cache_value) = self.get_command_cache_by_command_fingerprint(command_fingerprint).await? {
        // Command fingerprint matches, validate post-run fingerprint
        if let Some(post_run_mismatch) = cache_value.post_run_fingerprint.validate(fs, base_dir)? {
            Ok(Err(CacheMiss::FingerprintMismatch(
                FingerprintMismatch::PostRunFingerprintMismatch(post_run_mismatch),
            )))
        } else {
            // Cache hit, update association
            self.upsert_taskrun_to_command(&task_run_key, command_fingerprint).await?;
            Ok(Ok(cache_value))
        }
    } else if let Some(old_command_fp) = self.get_command_fingerprint_by_task_run_key(&task_run_key).await? {
        // Task run exists but command fingerprint changed
        Ok(Err(CacheMiss::FingerprintMismatch(
            FingerprintMismatch::CommandFingerprintMismatch(
                command_fingerprint.diff(&old_command_fp),
            ),
        )))
    } else {
        // No cache found
        Ok(Err(CacheMiss::NotFound))
    }
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

### Example: Task Run Key and Command Fingerprint

```rust
// Task: app#build --production
TaskRunKey {
    task_id: TaskId {
        task_group_id: TaskGroupId {
            task_group_name: "build".into(),
            is_builtin: false,
            config_path: RelativePathBuf::from("packages/app"),
        },
        subcommand_index: None,
    },
    args: vec!["--production"].into(),
}

CommandFingerprint {
    cwd: RelativePathBuf::from("packages/app"),
    command: TaskCommand::ShellScript("tsc && rollup -c".into()),
    envs_without_pass_through: btreemap! {
        "NODE_ENV".into() => "production".into()
    },
    pass_through_envs: btreeset! { "PATH".into(), "HOME".into() },
}
```

### Example: Built-in Task Cache Key

```rust
// Built-in task: vite-plus lint
TaskRunKey {
    task_id: TaskId {
        task_group_id: TaskGroupId {
            task_group_name: "lint".into(),
            is_builtin: true,
            config_path: RelativePathBuf::from("packages/frontend"), // Current working directory
        },
        subcommand_index: None,
    },
    args: vec![].into(),
}

CommandFingerprint {
    cwd: RelativePathBuf::from("packages/frontend"),
    command: TaskCommand::Parsed(TaskParsedCommand {
        program: "/usr/local/bin/oxlint".into(),
        args: vec![".".into()].into(),
        envs: HashMap::new(),
    }),
    envs_without_pass_through: BTreeMap::new(),
    pass_through_envs: btreeset! { "PATH".into() },
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

1. **NotFound**: No cache entry exists (first run or after cache clear)
2. **CommandFingerprintMismatch**: Command, args, environment variables, or pass-through config changed
3. **PostRunFingerprintMismatch**: Source files modified or file structure changed

#### Detailed Cache Miss Messages

From the test cases, cache miss messages include:

- `Cache miss: foo.txt content changed` - Input file content changed
- `Cache miss: Command fingerprint changed: CommandFingerprintDiff { ... }` - Command changed
- Pass-through env config change: `pass_through_envs: BTreeSetDiff { added: {}, removed: {"MY_ENV2"} }`
- Environment value change: `envs_without_pass_through: HashMapDiff { altered: {"FOO": Some("1")}, removed: {} }`

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

### 2. Shared Caching Across Tasks

Tasks with identical commands automatically share cache entries:

```json
{
  "scripts": {
    "script1": "cat foo.txt",
    "script2": "cat foo.txt"
  }
}
```

Behavior:

1. `vite run script1` creates command cache for `cat foo.txt`
2. `vite run script2` hits the same command cache (shared)
3. If `foo.txt` changes, both tasks will see cache miss on next run
4. Cache update from either task benefits the other

### 3. Individual Caching for Different Arguments

Tasks with different arguments get separate cache entries:

```bash
# These create separate caches
vite run echo -- a    # TaskRunKey with args: ["a"]
vite run echo -- b    # TaskRunKey with args: ["b"]
```

### 4. Compound Commands for Granular Caching

Leverage compound commands for per-subcommand caching:

```json
{
  "scripts": {
    "build": "tsc && rollup -c && terser dist/bundle.js"
  }
}
```

Benefit: Each `&&` separated command is cached independently. If only terser config changes, TypeScript and rollup will hit cache.

### 5. Disable Cache for Side Effects

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

### 6. File Access Patterns

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

## Cache Sharing Examples

### Example 1: Shared Command Cache

```bash
# Initial run creates command cache
> vite-plus run script1
Cache not found
bar

# Different task, same command - hits shared cache
> vite-plus run script2
Cache hit, replaying
bar

# File change invalidates shared cache
> echo baz > foo.txt
> vite-plus run script2
Cache miss: foo.txt content changed
baz

# Original task benefits from updated cache
> vite-plus run script1
Cache hit, replaying
baz
```

### Example 2: Individual Caching by Arguments

```bash
# Different args create separate caches
> vite-plus run echo -- a
Cache not found
a

> vite-plus run echo -- b
Cache not found
b

# Each argument combination has its own cache
> vite-plus run echo -- a
Cache hit, replaying
a

> vite-plus run echo -- b
Cache hit, replaying
b
```

### Example 3: Built-in Task Caching by Working Directory

```bash
# Different directories create separate caches for built-in tasks
> cd folder1 && vite-plus lint
Cache not found
Found 0 warnings and 0 errors.

> cd folder2 && vite-plus lint
Cache not found  # Different cwd = different cache
Found 0 warnings and 0 errors.

# Each directory maintains its own cache
> cd folder1 && vite-plus lint
Cache hit, replaying
Found 0 warnings and 0 errors.
```

## Implementation Reference

### Core Cache Components

```
┌──────────────────────────────────────────────────────────────┐
│                   Cache System Architecture                  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  crates/vite_task/src/                                       │
│  ├── cache.rs           # Two-tier cache storage system      │
│  │   ├── CommandCacheValue  # Cached execution results       │
│  │   ├── TaskRunKey        # Task run identification         │
│  │   ├── TaskCache         # Main cache interface            │
│  │   └── try_hit()         # Two-level cache lookup          │
│  │                                                           │
│  ├── fingerprint.rs     # Post-run fingerprint generation    │
│  │   ├── PostRunFingerprint     # Input file states         │
│  │   ├── PathFingerprint        # File/directory state       │
│  │   └── PostRunFingerprintMismatch # Validation results     │
│  │                                                           │
│  ├── config/mod.rs      # Command fingerprint generation     │
│  │   └── CommandFingerprint     # Command execution context  │
│  │                                                           │
│  ├── execute.rs         # Task execution with caching        │
│  │   ├── execute_with_cache() # Main execution flow          │
│  │   ├── monitor_files()      # fspy integration             │
│  │   └── capture_outputs()    # Output collection            │
│  │                                                           │
│  └── schedule.rs        # Task scheduling and cache lookup   │
│      └── schedule_tasks() # Cache-aware task execution       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Key Algorithms

#### Task Run Key Generation

```rust
// Generate task run key for cache lookup
impl TaskCache {
    pub async fn try_hit(&self, task: &ResolvedTask) -> Result<...> {
        let task_run_key = TaskRunKey {
            task_id: task.id(),
            args: task.args.clone(),
        };
        let command_fingerprint = &task.resolved_command.fingerprint;
        // ... two-tier lookup logic
    }
}
```

#### Post-Run Fingerprint Validation

```rust
// Validates cached post-run fingerprint against current file system state
impl PostRunFingerprint {
    pub fn validate(
        &self,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
    ) -> Result<Option<PostRunFingerprintMismatch>, Error> {
        let input_mismatch = self.inputs.par_iter().find_map_any(|(input_path, cached_fp)| {
            let full_path = base_dir.join(input_path);
            let current_fp = PathFingerprint::create(&full_path, fs);
            if cached_fp != &current_fp {
                Some(PostRunFingerprintMismatch::InputContentChanged {
                    path: input_path.clone(),
                })
            } else {
                None
            }
        });
        Ok(input_mismatch)
    }
}
```

### Performance Characteristics

- **Cache key generation**: ~1μs per task
- **File hashing**: ~10GB/s with xxHash3
- **Database operations**: <1ms for typical queries
- **Fingerprint validation**: ~10μs per task
- **Output replay**: Near-zero overhead

The cache system adds minimal overhead while providing significant speedups for unchanged tasks, making incremental builds in large monorepos extremely efficient.
