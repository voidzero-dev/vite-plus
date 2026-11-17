# create_workspace_install_failure

Creation reports a failing exit code when workspace dependency installation fails.

## `vpt json-edit package.json dependencies.missing-dependency file:./missing-dependency`


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`

**Exit code:** 1

```
● ➤ YN0000: · Yarn <version>
  ➤ YN0000: ┌ Resolution step
  ➤ YN0001: │ Error: missing-dependency@file:./missing-dependency::locator=plain-workspace%40workspace%3A.: ENOENT: no such file or directory, lstat '<workspace>/missing-dependency'
  ➤ YN0000: └ Completed
  ➤ YN0000: · Failed with errors in <duration>

■
● You may need to run "vp install" manually in <workspace>
◇ Scaffolded apps/web with Vite application
• Node <version>  yarn <version>
→ Next: cd apps/web && vp run
```
