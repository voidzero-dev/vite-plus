# migration_existing_vite_plus_esbuild_minify

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

Converge the project to a fully migrated Vite+ project; this pass rewrites the import and switches the minifier.


## `vpt replace-file-content vite.config.ts 'minify: true' 'minify: '\''esbuild'\'''`

Reintroduce the Vite 7-era esbuild minify as the project's ONLY pending change.


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

The static minify rewrite emits no warnings, so it must mark the migration as done instead of hitting the already-using-Vite+ fast exit.

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 1 config update applied
```

## `vpt grep-file vite.config.ts 'minify: true'`

The rewrite is applied to vite.config.

```
vite.config.ts: found "minify: true"
```
