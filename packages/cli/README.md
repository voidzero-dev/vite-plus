# Vite-Plus CLI Package

## Overview

This package provides the JavaScript-to-Rust bridge that enables vite-plus to execute JavaScript tooling (like Vite, Vitest, and oxlint) from the Rust core. It uses NAPI-RS to create native Node.js bindings.

## Architecture

### How It Works

The architecture follows a callback-based pattern where JavaScript functions resolve tool paths and pass them to Rust for execution:

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   JavaScript    │────▶│   NAPI Bridge    │────▶│    Rust Core    │
│   (bin.ts)      │     │   (binding/)     │     │  (vite_task)    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                        │                         │
        ▼                        ▼                         ▼
   Resolves tool            Converts JS              Executes tools
   binary paths           callbacks to Rust         with resolved paths
```

### Key Components

#### 1. JavaScript Layer (`src/`)

The JavaScript layer is responsible for resolving tool binary paths:

- **`bin.ts`**: Entry point that initializes the CLI with tool resolvers
- **`vite.ts`**: Resolves the Vite binary path for build commands
- **`test.ts`**: Resolves the Vitest binary path for test commands
- **`lint.ts`**: Resolves the oxlint binary path for linting
- **`index.ts`**: Exports the `defineConfig` helper for Vite configuration

Each resolver function returns:

```typescript
{
  binPath: string,    // Absolute path to the tool's binary
  envs: Record<string, string>  // Environment variables to set
}
```

#### 2. NAPI Binding Layer (`binding/`)

The binding layer provides the JavaScript-to-Rust bridge using NAPI-RS:

- **`src/lib.rs`**: Defines the NAPI bindings and type conversions
- **`index.d.ts`**: TypeScript type definitions (auto-generated)
- **`index.js`**: Native module loader (auto-generated)

The binding converts JavaScript callbacks into Rust futures using `ThreadsafeFunction`.

#### 3. Rust Core Integration

The Rust core (`crates/vite_task`) receives the tool resolvers through `CliOptions`:

```rust
pub struct CliOptions {
    pub lint: LintFn,  // Callback to resolve lint tool
    pub vite: ViteFn,  // Callback to resolve vite tool
    pub test: TestFn,  // Callback to resolve test tool
}
```

## Execution Flow

1. **Initialization**: `bin.ts` calls `run()` with tool resolver functions
2. **Command Parsing**: Rust parses CLI arguments to determine which command to run
3. **Tool Resolution**: When a command needs a tool (e.g., `vite build`):
   - Rust calls back to JavaScript through NAPI
   - JavaScript resolver finds the tool's binary path
   - Path is returned to Rust
4. **Execution**: Rust executes the tool binary with appropriate arguments

## Example: Vite Build Command

When a user runs `vite-plus build`:

1. Rust identifies this as a Build command
2. Calls the `vite` callback function
3. JavaScript `vite.ts` resolves `vite/bin/vite.js` path
4. Returns path to Rust
5. Rust executes: `node /path/to/vite.js build [args]`

## Development

### Building

```bash
# Build the native binding
pnpm build

# Or watch for changes
pnpm build:debug
```

### Adding a New Tool

1. Create a resolver in `src/`:

```typescript
// src/mytool.ts
export async function mytool() {
  const binPath = require.resolve('mytool/bin/cli.js');
  return { binPath, envs: {} };
}
```

2. Add to `CliOptions` in `binding/src/lib.rs`:

```rust
pub struct CliOptions {
    // ... existing fields
    pub mytool: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
}
```

3. Wire it up in `bin.ts`:

```typescript
import { mytool } from './mytool.js';
run({ lint, vite, test, mytool });
```

## Benefits of This Architecture

1. **Tool Resolution in JavaScript**: Leverages Node.js module resolution to find tools
2. **Execution in Rust**: Benefits from Rust's performance and concurrency
3. **Type Safety**: Full type safety across the JS-Rust boundary
4. **Flexibility**: Easy to add new tools without changing core logic
5. **Environment Handling**: Can pass environment variables per tool

## Dependencies

- `napi`: Node-API bindings for Rust
- `napi-derive`: Procedural macros for NAPI
- `vite`, `vitest`, `oxlint`: The actual tools being wrapped
