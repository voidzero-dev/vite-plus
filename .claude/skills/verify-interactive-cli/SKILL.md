---
name: verify-interactive-cli
description: Drive and capture vp's interactive (clack) prompts in a tmux session to manually verify interactive UX and catch spinner-over-prompt bugs. Use when asked to test/verify/capture an interactive vp command's prompts (vp migrate, vp create, ...), reproduce a prompt-rendering bug, or show the real interactive CLI output for a PR.
allowed-tools: Bash, Read
---

# Verify vp's interactive CLI prompts

The PTY snapshot suite covers scripted prompt flows. This skill complements it by driving an installed CLI in tmux, capturing clean output, and checking a prompt over several seconds for a spinner animating underneath it (a real, recurring UX bug class, e.g. the "Preparing migration" and "Checking Node.js version support" spinners).

## Prerequisites

- `tmux` (macOS has none by default): `brew install tmux`.
- The global `vp` must contain the code under test. To exercise working-tree changes, rebuild first: `pnpm bootstrap-cli` (~5 min), then confirm with `vp --version`.

## Driver

`interactive-cli-tmux-driver.sh` is bundled in this skill's directory.

```bash
# Run to completion, auto-accepting every prompt's DEFAULT; prints a clean transcript.
.claude/skills/verify-interactive-cli/interactive-cli-tmux-driver.sh <project-dir> "vp migrate"

# STOP at a specific prompt (do NOT answer it) and check for a spinner animating under it.
.claude/skills/verify-interactive-cli/interactive-cli-tmux-driver.sh <project-dir> "vp migrate" "Upgrade Node.js"
```

How it works:

- Runs the command in a detached tmux session; `tmux capture-pane -p` yields clean text (clack's in-place redraws overwrite, so only resolved lines remain), far cleaner than `expect`'s raw ANSI capture.
- Auto-accepts each prompt's default by sending Enter when the pane goes STABLE. A waiting prompt is static; animating spinners keep the pane changing, so Enter never fires mid-work.
- End-detection uses `; echo "$M1$M2 exit=$?"` where M1/M2 are split vars, so the literal end marker is not in the typed command line (otherwise a grep matches the echoed command, not the program output).
- With a STOP_AT regex it halts at the target prompt and captures twice ~3s apart: identical captures = static prompt (OK); differing captures (or a `Checking ... (Xs)` line) = a spinner is animating under the prompt = a UX bug.

## Setting up a target project

Create a throwaway project that triggers the prompts you want to see, e.g. a fresh Vite app whose `.node-version` is below the supported range to exercise the Node-upgrade confirm:

```bash
mkdir -p /tmp/vp-demo && cd /tmp/vp-demo
printf '{\n  "name":"d","private":true,"type":"module","packageManager":"pnpm@10.18.0",\n  "scripts":{"build":"vite build","test":"vitest run"},\n  "devDependencies":{"vite":"^8.0.0","vitest":"^4.1.0"}\n}\n' > package.json
echo "24.3.0" > .node-version
printf 'import { defineConfig } from "vite";\nexport default defineConfig({});\n' > vite.config.ts
git init -q && git add -A && git -c user.email=x@x -c user.name=x commit -qm init
```

## When you find a spinner-over-prompt bug

Fix it test-first: the prompt's code must pause the migration progress spinner before the confirm renders. Assert the call order is `['pause', 'confirm']`. Reference fix: `upgradeUnsupportedNodeVersions` (in `packages/cli/src/migration/migrator/setup.ts`) takes a `pauseProgress` callback that `bin.ts` wires to `clearMigrationProgress`, called right before `prompts.confirm`.

## Gotchas

- clack's active-prompt marker here is `›` (not always `◆`); detect a waiting prompt by pane stability, not a specific glyph.
- `VP_SKIP_INSTALL=1` skips the dependency install but breaks steps that load `vite.config.ts` (e.g. the prettier auto-migration) because `vite` isn't installed; use it only when you stop before the install step.
- A `&` inside a background task detaches the script, so the task reports "completed" early while it keeps running; poll a status file or `capture-pane` to track real progress.
