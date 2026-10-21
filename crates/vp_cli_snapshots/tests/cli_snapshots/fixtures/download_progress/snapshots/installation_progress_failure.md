# installation_progress_failure

## `node install.mjs interactive failure`

**→ expect-milestone:** `install:download:ready`

```
Before installation: preserve this output.
info: installing vite-plus@<version>...
⠿ Downloading pnpm <version>... <size> B (<size> B/s)
```

**← write-key:** `enter`

**→ expect-milestone:** `install:prepare:ready`

```
Before installation: preserve this output.
info: installing vite-plus@<version>...
⠿ Preparing Node.js and pnpm... <duration>
```

**← write-key:** `enter`

**→ expect-milestone:** `install:dependencies:ready`

```
Before installation: preserve this output.
info: installing vite-plus@<version>...
⠿ Installing dependencies... <duration>
```

**← write-key:** `enter`

```
Before installation: preserve this output.
info: installing vite-plus@<version>...
error: Setup error: Failed to install production dependencies (exit code: 17). See log for details: <workspace>/home/upgrade.log
Failure log preserves pnpm stdout and stderr.
After installation.
```
