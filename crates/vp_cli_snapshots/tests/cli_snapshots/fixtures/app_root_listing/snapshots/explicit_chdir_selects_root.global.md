# explicit_chdir_selects_root

An explicit `-C .` selects the workspace root. The command does not open the
package picker again after the CLI consumes `-C`.

## `vp -C . build`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
✓ 2 modules transformed.
computing gzip size...
dist/assets/root-entry-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
