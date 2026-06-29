# Check Config

`vp check` runs format, lint, and type checks together. The `check` block in `vite.config.ts` sets defaults for the composite command, mirroring the `--no-fmt` and `--no-lint` CLI flags.

This is useful when a project wants to keep most of the toolchain but skip one step by default. For example, a team that lints but does not format can disable `check.fmt` so a plain `vp check` (the command agents and contributors run most) only lints, without anyone needing to remember `--no-fmt`.

## Example

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  check: {
    // Skip the format step in `vp check`. Defaults to true.
    fmt: false,
    // Skip lint rules in `vp check`. Type-check still runs when both
    // `lint.options.typeAware` and `lint.options.typeCheck` are enabled.
    // Defaults to true.
    lint: true,
  },
});
```

When a step is disabled here, `vp check` prints a short `note:` line so it is clear why the step did not run. With the `check.fmt: false` config above:

```bash
$ vp check
note: Format skipped (check.fmt: false in vite.config.ts)
pass: Found no warnings or lint errors in 1 file (12ms, 8 threads)
```

## Scope and precedence

- These options only affect the composite `vp check`. Standalone [`vp fmt`](/config/fmt) and [`vp lint`](/config/lint) are unaffected, so you can still run a disabled tool directly when you need it once. Note that any `vp check` invocation honors these defaults, including one run from a pre-commit hook: if your [`staged`](/config/staged) tasks call `vp check`, that step is skipped there too.
- A step is skipped if the config disables it **or** the matching CLI flag is passed. There is no flag to re-enable a step disabled in config; run `vp fmt` or `vp lint` directly instead.
