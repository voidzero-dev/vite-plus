# migration_oxc_ecosystem_sigle

Reproduces the two app-level JSON extends configs in https://github.com/vite-plus-ecosystem-ci/sigle/pull/8, with a shared base to verify inherited rules.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file .oxlintrc.json`

```
{
  "rules": {
    "no-console": "error"
  }
}
```

## `vpt print-file apps/sigle/.oxlintrc.json`

```
{
  "extends": ["../../.oxlintrc.json"],
  "rules": {
    "no-alert": "off"
  }
}
```

## `vpt print-file apps/custom-domain/.oxlintrc.json`

```
{
  "extends": ["../../.oxlintrc.json"],
  "rules": {
    "no-alert": "off"
  }
}
```

## `vpt print-file apps/sigle/vite.config.ts`

JSON extends paths must not be copied into the existing Vite config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    tsconfigPaths: true,
  },
});
```

## `vpt stat-file apps/custom-domain/vite.config.ts --assert-not file`

an app without a Vite config must keep its JSON config

```
apps/custom-domain/vite.config.ts: missing
```

## `cd apps/sigle && vp lint -c .oxlintrc.json src`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd apps/custom-domain && vp lint --config .oxlintrc.json src`

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file apps/sigle/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    tsconfigPaths: true,
  },
});
```

## `vpt stat-file apps/custom-domain/vite.config.ts --assert-not file`

```
apps/custom-domain/vite.config.ts: missing
```

## `vpt write-file apps/sigle/src/index.ts 'console.log('\''hello'\'');
'`


## `vpt write-file apps/custom-domain/src/index.ts 'console.log('\''hello'\'');
'`


## `cd apps/sigle && vp lint -c .oxlintrc.json src`

the app must still load its inherited JSON rule

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello');
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd apps/custom-domain && vp lint --config .oxlintrc.json src`

the app without vite.config.ts must also load its inherited JSON rule

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[src/index.ts:1:1]
 1 │ console.log('hello');
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
