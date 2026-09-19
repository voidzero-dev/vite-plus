# legacy_test_entry_points

Migrate legacy test aliases on fresh and existing Vite+ projects. Check runtime exports and a no-op rerun.

## `vpt cp entries.mjs entries.before`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 1 file had imports rewritten
```

## `vpt print-file entries.mjs`

```
import { BaseCoverageProvider } from "vite-plus/test/node";
export { DefaultReporter } from "vite-plus/test/node";
export * as environments from 'vite-plus/test/runtime';
export { VitestSnapshotEnvironment } from "vite-plus/test/runtime";
export * as mocker from 'vite-plus/test/mocker';

export { BaseCoverageProvider };
export const loadEnvironment = () => import('vite-plus/test/runtime');
```

## `node verify.mjs`

```
Canonical entry points load; legacy aliases are absent; mocker remains available.
```

## `vpt cp entries.before entries.mjs`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 1 file had imports rewritten
```

## `vpt print-file entries.mjs`

```
import { BaseCoverageProvider } from 'vite-plus/test/node';
export { DefaultReporter } from 'vite-plus/test/node';
export * as environments from 'vite-plus/test/runtime';
export { VitestSnapshotEnvironment } from 'vite-plus/test/runtime';
export * as mocker from 'vite-plus/test/mocker';

export { BaseCoverageProvider };
export const loadEnvironment = () => import('vite-plus/test/runtime');
```

## `node verify.mjs`

```
Canonical entry points load; legacy aliases are absent; mocker remains available.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
