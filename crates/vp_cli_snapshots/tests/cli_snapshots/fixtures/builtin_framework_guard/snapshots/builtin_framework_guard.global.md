# builtin_framework_guard

## `vp dev`

`vp dev` refuses in a Nuxt project and points at the dev script

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp dev` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run dev`?
```

## `vp build`

`vp build` refuses the same way

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp build` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run build`?
```

## `cd src && vp dev`

the refusal finds the enclosing package from a subdirectory, with the same walk `vp run` uses

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp dev` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run dev`?
```

## `cd astro && vp dev`

an Astro config triggers the same refusal

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Astro (astro.config.mjs). `vp dev` runs the bundled Vite CLI, not the Astro CLI.
hint: did you mean `vp run dev`?
```

## `cd no-scripts && vp dev`

without scripts, the hint points at the framework CLI through `vp exec`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp dev` runs the bundled Vite CLI, not the Nuxt CLI.
hint: run the Nuxt CLI with `vp exec nuxt dev`.
```

## `cd no-scripts && vp build`

`vp build` gets the same fallback hint

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp build` runs the bundled Vite CLI, not the Nuxt CLI.
hint: run the Nuxt CLI with `vp exec nuxt build`.
```

## `cd renamed-script && vp dev`

a script that runs the framework dev command under another name becomes the hint target

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp dev` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run start`? The start script runs `nuxi dev`.
```

## `cd renamed-script && vp build`

the build hint finds the renamed build script the same way

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp build` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run make`? The make script runs `nuxt build`.
```

## `cd mismatched-script && vp dev`

an unrelated same-named script does not become the hint; the script that runs nuxt dev does

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: this project uses Nuxt (nuxt.config.ts). `vp dev` runs the bundled Vite CLI, not the Nuxt CLI.
hint: did you mean `vp run serve`? The serve script runs `nuxt dev`.
```

## `vp build web`

a positional root is inspected instead of the invocation package: the plain Vite child builds

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
note: `vp build web` sets Vite's root without changing the working directory. To run as if started there, use `vp -C web build`.
✓ 4 modules transformed.
computing gzip size...
web/dist/index.html                <size> kB │ gzip: <size> kB
web/dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp dev astro`

a positional root inside an Astro package refuses, and the hint carries -C

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

note: `vp dev astro` sets Vite's root without changing the working directory. To run as if started there, use `vp -C astro dev`.
error: this project uses Astro (astro.config.mjs). `vp dev` runs the bundled Vite CLI, not the Astro CLI.
hint: did you mean `vp -C astro run dev`?
```

## `vp dev --config vite.config.ts --port 12312312312`

an explicit --config selects the bundled Vite CLI on purpose, so only the script note prints (the invalid port stops the server immediately)

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, use `vpr dev` instead.
error when starting dev server:
Error: No available ports found between 12312312312 and 65535
```

## `vp dev --help`

a help request reaches the tool, so the guard steps aside

```
VITE+ - The Unified Toolchain for the Web

Usage: vp dev [ROOT] [OPTIONS]

Run the development server.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [host]           [string] specify hostname
  --port <port>           [number] specify port
  --open [path]           [boolean | string] open browser on startup
  --cors                  [boolean] enable CORS
  --strictPort            [boolean] exit if specified port is already in use
  --force                 [boolean] force the optimizer to ignore the cache and re-bundle
  --experimentalBundle    [boolean] use experimental full bundle mode (this is highly experimental)
  --base <path>           [string] public base path (default: /)
  -l, --logLevel <level>  [string] info | warn | error | silent
  --clearScreen           [boolean] allow/disable clear screen when logging
  -d, --debug [feat]      [string | boolean] show debug logs
  -f, --filter <filter>   [string] filter debug logs
  -m, --mode <mode>       [string] set env mode
  -h, --help              Display this message

Examples:
  vp dev
  vp dev --open
  vp dev --host localhost --port 5173

Documentation: https://viteplus.dev/guide/dev
```

## `vp run dev`

`vp run dev` runs the dev script that the refusal points at

```
VITE+ - The Unified Toolchain for the Web

$ vpt print nuxt dev script ⊘ cache disabled
nuxt dev script
```
