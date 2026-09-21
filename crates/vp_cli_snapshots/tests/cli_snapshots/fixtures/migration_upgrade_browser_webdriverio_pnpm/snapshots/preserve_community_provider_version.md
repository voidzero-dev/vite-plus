# preserve_community_provider_version

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`


## `vpt json-edit package.json devDependencies.@vitest/browser-webdriverio ^5.0.0`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "@vitest/browser-webdriverio": "^5.0.0",
    "vite-plus": "catalog:",
    "vitest": "catalog:",
    "webdriverio": "*"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "onFail": "download",
      "version": "10.33.0"
    }
  },
  "name": "migration-upgrade-browser-webdriverio-pnpm"
}
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';
import { webdriverio } from '@vitest/browser-webdriverio';

export default defineConfig({
  test: {
    browser: {
      enabled: true,
      provider: webdriverio(),
    },
  },
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
