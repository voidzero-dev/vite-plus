# RFC: Interactive CLI Testing with PTY

## Background

The vite-plus CLI has several interactive features that currently have **zero integration test coverage**:

- **Rust (global CLI)**: Command picker (`command_picker.rs`) using `crossterm` raw terminal mode with keyboard navigation, search filtering, and alternate screen. Confirmation prompts in `implode.rs` and `env/pin.rs` using `stdin.read_line()`.
- **TypeScript (local CLI)**: `@clack/core`-based prompts in `packages/prompts/` — select, multiselect, confirm, text, password, autocomplete. Used by `vp create` (package name, target dir, template selection) and other interactive flows.

Current snap tests bypass all interactivity via `stdin: null` and `CI=true`. The only testing for prompts is render-function-level unit tests (`packages/prompts/src/__tests__/render.spec.ts`) that mock `@clack/core` and assert on rendered strings — they do not test actual user interaction flows (keystroke → state change → re-render → final selection).

## Problem

1. **No coverage of interactive flows**: Bugs in keyboard handling, state transitions, or prompt sequencing are invisible to tests
2. **AI/automation blind spot**: Interactive CLI behavior cannot be verified by AI agents or CI pipelines
3. **Regression risk**: Changes to `crossterm` event handling or `@clack/core` prompt logic could break the user experience silently

## Goals

1. Enable automated testing of interactive CLI flows end-to-end
2. Preserve both **inputs** (keystrokes) and **outputs** (terminal renders) in snap text files for diff-based review
3. Reuse the existing snap test UX: directory-based test cases, `steps.json` configuration, `snap.txt` output, `git diff` for verification
4. Support both Rust global CLI (crossterm) and TypeScript local CLI (@clack/core prompts)

## Non-Goals

- Replace existing non-interactive snap tests
- Test every possible prompt state combination (unit render tests cover that)
- Support Windows PTY testing in the initial implementation (can be added later)

## Design

### Architecture

```
┌─────────────────────┐     spawn in PTY      ┌──────────────┐
│  Test Runner        │ ──────────────────────→│  CLI Process │
│  (snap-test.ts)     │                        │  (vp ...)    │
│                     │     PTY stdout/stdin    │              │
│  expect("Select")───┼───── read ←────────────┤  crossterm / │
│  send(↓)  ──────────┼───── write ───────────→│  @clack/core │
│  expect("migrate")──┼───── read ←────────────┤              │
│  send(Enter) ───────┼───── write ───────────→│              │
│                     │                        └──────────────┘
│  capture terminal   │
│  buffer → snap.txt  │
└─────────────────────┘
```

A PTY (pseudo-terminal) is required because:

- `crossterm` checks `stdin.is_terminal()` and `stdout.is_terminal()` — pipes won't work
- `@clack/core` checks `output.isTTY` for rendering behavior
- Raw terminal mode, alternate screen, and cursor control only work in a real TTY

### PTY Library

Use **`node-pty`** (npm package) for the test runner. It works for both Rust and TypeScript CLI testing since we spawn the `vp` binary as a subprocess regardless of implementation language.

Alternatives considered:

- Rust `expectrl`/`rexpect`: Only useful for Rust-side unit tests, doesn't help with TypeScript flows
- Custom `child_process` with pipes: Won't satisfy `is_terminal()` checks

### Test Case Structure

Interactive snap tests live alongside existing snap tests in a new directory:

```
packages/cli/snap-tests-interactive/
  command-picker-select/
    steps.json
    snap.txt
  command-picker-search/
    steps.json
    snap.txt
  create-project-flow/
    steps.json
    snap.txt
  implode-confirm/
    steps.json
    snap.txt
```

### steps.json Format

Extend the existing `steps.json` format with an `interactive` command type:

```jsonc
{
  "env": {},
  "commands": [
    {
      "command": "vp",
      "interactive": true,
      "cols": 80,
      "rows": 24,
      "steps": [
        // Wait for text to appear, then send keystrokes
        { "expect": "Select a command", "send": "\u001b[B" }, // Arrow Down
        { "expect": "migrate", "send": "\u001b[B" }, // Arrow Down
        { "expect": "dev", "send": "\r" }, // Enter
        { "expect": "exit" }, // Wait for process exit
      ],
      "timeout": 30000,
    },
  ],
}
```

#### Step Types

| Field      | Type      | Description                                                          |
| ---------- | --------- | -------------------------------------------------------------------- |
| `expect`   | `string`  | Text pattern to wait for in PTY output before proceeding             |
| `send`     | `string`  | Keystrokes to send after expect matches. Supports escape sequences   |
| `snapshot` | `boolean` | If true, capture terminal buffer state at this point (default: true) |
| `delay`    | `number`  | Optional delay (ms) before sending input, for debounced prompts      |

#### Common Key Sequences

```jsonc
{
  "keys": {
    "\\u001b[A": "Arrow Up",
    "\\u001b[B": "Arrow Down",
    "\\r": "Enter",
    "\\u001b": "Escape",
    "\\u0003": "Ctrl+C",
    "\\u007f": "Backspace",
  },
}
```

### snap.txt Format

The snap text captures a **transcript** of the interactive session, showing both inputs and terminal state at each step:

```
> vp (interactive, 80x24)

--- expect: "Select a command" ---

  Vite+

  Select a command (↑/↓, Enter to run, type to search):

  › create: Create a new project from a template.
    migrate
    dev
    check
    test
    install
    run
    build
    pack
    preview
    config
    outdated
    env
    help

--- send: Down ---
--- expect: "migrate" ---

  Vite+

  Select a command (↑/↓, Enter to run, type to search):

    create
  › migrate: Migrate an existing project to Vite+.
    dev
    check
    test
    install
    run
    build
    pack
    preview
    config
    outdated
    env
    help

--- send: Down ---
--- expect: "dev" ---

  Vite+

  Select a command (↑/↓, Enter to run, type to search):

    create
    migrate
  › dev: Run the development server.
    ...

--- send: Enter ---
--- expect: "exit" ---
```

Key properties of this format:

- **Human-readable**: Reviewers can see exactly what the user would see at each step
- **Diff-friendly**: Changes to prompt text, ordering, or behavior show up clearly in `git diff`
- **Input-annotated**: `--- send: ... ---` lines document what keystroke triggered the next state
- **ANSI-stripped**: All color/cursor codes removed for stable text comparison

### Terminal Buffer Capture

Instead of capturing raw PTY byte stream (which includes cursor movements, screen clears, and partial redraws), we use a **virtual terminal emulator** to maintain a screen buffer and capture the final rendered state at each step.

Use **`xterm-headless`** (from the xterm.js project) as the virtual terminal:

```typescript
import { Terminal } from '@xterm/headless';

const term = new Terminal({ cols: 80, rows: 24 });
ptyProcess.onData((data) => {
  term.write(data);
});

// At snapshot point: read the screen buffer
function captureScreen(term: Terminal): string {
  const lines: string[] = [];
  for (let i = 0; i < term.rows; i++) {
    const line = term.buffer.active.getLine(i);
    if (line) {
      lines.push(line.translateToString(true)); // true = trim trailing whitespace
    }
  }
  // Trim trailing empty lines
  while (lines.length > 0 && lines[lines.length - 1].trim() === '') {
    lines.pop();
  }
  return lines.join('\n');
}
```

This approach gives us a clean, stable representation of what the user actually sees on screen.

### Test Runner Integration

Add a new function `runInteractiveCommand()` in `packages/tools/src/snap-test.ts`:

```typescript
import { spawn } from 'node-pty';
import { Terminal } from '@xterm/headless';

interface InteractiveStep {
  expect: string;
  send?: string;
  snapshot?: boolean;
  delay?: number;
}

interface InteractiveCommand {
  command: string;
  interactive: true;
  cols?: number;
  rows?: number;
  steps: InteractiveStep[];
  timeout?: number;
}

async function runInteractiveCommand(
  cmd: InteractiveCommand,
  env: Record<string, string>,
  cwd: string,
): Promise<string[]> {
  const cols = cmd.cols ?? 80;
  const rows = cmd.rows ?? 24;
  const snapLines: string[] = [];

  // Remove CI and NO_COLOR so interactive mode activates
  const interactiveEnv = { ...env };
  delete interactiveEnv['CI'];
  delete interactiveEnv['NO_COLOR'];
  // Force non-CI interactive mode
  interactiveEnv['TERM'] = 'xterm-256color';

  const ptyProcess = spawn(cmd.command.split(' ')[0], cmd.command.split(' ').slice(1), {
    name: 'xterm-256color',
    cols,
    rows,
    cwd,
    env: interactiveEnv,
  });

  const vterm = new Terminal({ cols, rows });
  ptyProcess.onData((data) => vterm.write(data));

  snapLines.push(`> ${cmd.command} (interactive, ${cols}x${rows})`);
  snapLines.push('');

  for (const step of cmd.steps) {
    // Wait for expected text
    await waitForText(vterm, step.expect, cmd.timeout ?? 30000);

    const shouldSnapshot = step.snapshot !== false;
    if (shouldSnapshot) {
      snapLines.push(`--- expect: "${step.expect}" ---`);
      snapLines.push('');
      snapLines.push(captureScreen(vterm));
      snapLines.push('');
    }

    if (step.send !== undefined) {
      if (step.delay) {
        await setTimeout(step.delay);
      }
      ptyProcess.write(step.send);
      snapLines.push(`--- send: ${describeKey(step.send)} ---`);
    }
  }

  ptyProcess.kill();
  return snapLines;
}
```

### Running Interactive Tests

```bash
# Run all interactive snap tests
pnpm -F vite-plus snap-test-interactive

# Run specific test
pnpm -F vite-plus snap-test-interactive command-picker
```

Interactive tests should run **serially** (not in parallel) since PTY tests are more resource-intensive and timing-sensitive.

### CI Considerations

- **GitHub Actions**: PTY works out of the box on Linux and macOS runners (both have `/dev/ptmx`)
- **Windows**: `node-pty` supports Windows via ConPTY, but interactive behavior may differ. Start with `"ignoredPlatforms": ["win32"]` and add Windows support later
- **Timeout**: Interactive tests need longer timeouts (30s default vs 50s for regular snap tests) since they involve multiple expect/send round-trips
- **Determinism**: Use fixed terminal size (`80x24`), strip ANSI codes, and use `xterm-headless` for consistent screen capture across environments

## Implementation Plan

### Phase 1: Infrastructure

1. Add `node-pty` and `@xterm/headless` as dev dependencies in `packages/tools/`
2. Implement `runInteractiveCommand()` in `packages/tools/src/snap-test.ts`
3. Implement `captureScreen()` using `@xterm/headless` Terminal buffer
4. Implement `waitForText()` with timeout and polling
5. Implement `describeKey()` to convert escape sequences to human-readable names
6. Add `snap-test-interactive` script to `packages/cli/package.json`
7. Extend `replaceUnstableOutput()` with interactive-specific normalizations if needed

### Phase 2: First Test Cases

8. `command-picker-select`: Launch `vp` with no args → picker appears → navigate → select
9. `command-picker-search`: Launch `vp` → type search query → filtered list → select
10. `command-picker-cancel`: Launch `vp` → press Escape → verify exit

### Phase 3: TypeScript Interactive Tests

11. `create-project-interactive`: `vp create --interactive` → fill prompts → verify project created
12. `implode-confirm`: `vp implode` → type "uninstall" → verify (with mock home dir)

### Phase 4: CI Integration

13. Add interactive snap tests to GitHub Actions workflow
14. Configure platform-specific skipping for Windows

## Key Files to Modify

| File                                   | Change                                                        |
| -------------------------------------- | ------------------------------------------------------------- |
| `packages/tools/package.json`          | Add `node-pty`, `@xterm/headless` dependencies                |
| `packages/tools/src/snap-test.ts`      | Add `runInteractiveCommand()`, integrate with existing runner |
| `packages/tools/src/utils.ts`          | Add interactive-specific output normalizations                |
| `packages/cli/package.json`            | Add `snap-test-interactive` script                            |
| `packages/cli/snap-tests-interactive/` | New test case directories                                     |

## Existing Code to Reuse

- `replaceUnstableOutput()` from `packages/tools/src/utils.ts` — for normalizing paths, versions, dates
- `runWithConcurrencyLimit()` from `packages/tools/src/snap-test.ts` — serial execution with concurrency=1
- `isPassThroughEnv()` from `packages/tools/src/utils.ts` — environment variable filtering
- `steps.json` / `snap.txt` file structure and conventions from existing snap tests

## Verification

After implementing, verify with:

1. Run `pnpm -F vite-plus snap-test-interactive` — all test cases should generate `snap.txt`
2. Review generated `snap.txt` files — they should show readable interactive transcripts
3. Run `git diff` — verify snap text is stable across consecutive runs
4. Intentionally modify a prompt (e.g., change command label) — verify `git diff` shows the change clearly
5. Run on CI — verify tests pass on Linux and macOS runners
