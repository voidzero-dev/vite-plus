# create_next_command_application_from_package_manager

`npm create vite-plus` stamps npm's user agent: create with npm and run vp through it, since the global CLI may be missing.

## `vp create vite:application --no-interactive`

verify npm is selected and the next command uses npx

```
● Using default package name: vite-plus-application
◇ Scaffolded vite-plus-application with Vite application
• Node <version>  npm <version>
→ Next: cd vite-plus-application && npx vp run
```
