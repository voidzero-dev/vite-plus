---
name: vite-plus
description: >-
  Use the Vite+ (vp) CLI toolchain to run, build, test, lint, format, and type check web applications.
---

# Vite+ (`vp` CLI) Web Development Skill

This skill allows the agent to build, run, format, lint, and type check web projects using the Vite+ (`vp`) toolchain. 

> [!IMPORTANT]
> For any Vite+ project, the agent **MUST** prefer using the `vp` CLI commands over traditional `npm`, `yarn`, `pnpm`, or custom `package.json` scripts (like `npm run build`, `npm run lint`, etc.).

## Core Commands

The `vp` tool consolidates multiple frontend development utilities into one command-line interface:

### 1. Unified Project Checking (`vp check`)
Instead of running separate formatting, linting, and TypeScript checks, use `vp check` to run all static checks simultaneously.
- **Run all checks:**
  ```bash
  vp check
  ```
- **Automatically fix linting and formatting issues:**
  ```bash
  vp check --fix
  ```
- **Skip specific checks:**
  ```bash
  vp check --no-fmt
  vp check --no-lint
  ```

### 2. Project Production Build (`vp build`)
Build the project for production using the Rolldown/Oxc toolchain.
- **Run build:**
  ```bash
  vp build
  ```

### 3. Local Development Server (`vp dev`)
Run the local Vite-based development server.
- **Run dev server:**
  ```bash
  vp dev
  ```

### 4. Run Tests (`vp test`)
Execute tests in the project using Vitest.
- **Run all tests:**
  ```bash
  vp test
  ```

### 5. Install Dependencies (`vp install`)
Installs project dependencies using the auto-detected package manager.
- **Install packages:**
  ```bash
  vp install
  ```

### 6. Dynamic Script Execution (`vp run`)
Run any custom package script or command with monorepo dependency caching.
- **Run custom script:**
  ```bash
  vp run <command>
  ```

## Common Pitfalls

- **Using npm scripts directly:** Do not use `npm run build` or `npm run lint`. Instead, use `vp build` and `vp check`.
- **Target Directories:** Always ensure you are running `vp` commands in the directory containing the Vite+ config (e.g., the `web` directory in monorepos).
