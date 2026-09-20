# installation_progress_interactive

## `node install.mjs interactive`

**→ expect-milestone:** `install:prepare:ready`

```
Before installation: preserve this output.
info: installing vite-plus@0.3.3...
⠿ Preparing Node.js and pnpm... <duration>
```

**← write-key:** `enter`

**→ expect-milestone:** `install:download:ready`

```
Before installation: preserve this output.
info: installing vite-plus@0.3.3...
⠿ Downloading pnpm <version>... <size> B (<size> B/s)
```

**← write-key:** `enter`

**→ expect-milestone:** `install:dependencies:ready`

```
Before installation: preserve this output.
info: installing vite-plus@0.3.3...
⠿ Installing dependencies... <duration>
```

**← write-key:** `enter`

```
Before installation: preserve this output.
info: installing vite-plus@0.3.3...
✓ Dependencies installed.
Setup:
  Preparing vite-plus environment.

Created Shims:
  <workspace>/home/bin/node
  <workspace>/home/bin/npm
  <workspace>/home/bin/npx
  <workspace>/home/bin/pnpm
  <workspace>/home/bin/pnpx
  <workspace>/home/bin/yarn
  <workspace>/home/bin/yarnpkg
  <workspace>/home/bin/bun
  <workspace>/home/bin/bunx
  <workspace>/home/bin/vpx
  <workspace>/home/bin/vpr

Next Steps:
  Activate Vite+ in this terminal:
  . "<workspace>/home/env"

  Add the command to your .zshrc file to activate future Zsh terminals.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
✓ Vite+ setup complete.
Bootstrap stdout contains only shell assignments.
After installation.
```
