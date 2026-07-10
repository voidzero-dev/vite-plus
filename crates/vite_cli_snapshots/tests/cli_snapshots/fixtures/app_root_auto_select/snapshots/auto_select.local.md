# auto_select

With exactly one likely-runnable package, a bare app command in an interactive
terminal auto-selects it, prints the Selected/Tip teaching lines, and runs
there (rfcs/cwd-flag.md). This TTY-only branch was untestable in the legacy
snap runner.

## `vp build`

```
Selected package: web (apps/web)
Tip: run this directly with `vp -C apps/web build`
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
