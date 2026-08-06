# create_monorepo_local_template_shorthand

## `vp create starter --no-interactive --no-agent -- --directory my-app`

run the local create.templates entry; generated pkg declares fmt/lint via shorthand

```

Generating project…

Running: node <workspace>/packages/starter-template/bin/index.mjs --directory my-app
cloned starter-template to my-app

Monorepo integration...

lint config already present in packages/my-app/vite.config.ts — removed redundant packages/my-app/.oxlintrc.json

fmt config already present in packages/my-app/vite.config.ts — removed redundant packages/my-app/.oxfmtrc.json

Formatting code...

Code formatted
◇ Scaffolded packages/my-app
• Node <version>  pnpm <version>
→ Next: cd packages/my-app && vp run
```

## `vpt print-file packages/my-app/vite.config.ts`

fmt/lint stay shorthand only, no injected duplicate inline fmt:/lint: blocks (#1836)

```
import { defineConfig } from "vite-plus";

import { fmt } from "./tooling/format";
import { lint } from "./tooling/lint";

export default defineConfig(({ mode }) => {
  return {
    server: { port: 3000 },
    fmt,
    lint,
  };
});
```

## `vpt stat-file packages/my-app/.oxlintrc.json --assert-not file`

standalone lint config merge-skipped and removed

```
packages/my-app/.oxlintrc.json: missing
```

## `vpt stat-file packages/my-app/.oxfmtrc.json --assert-not file`

standalone fmt config merge-skipped and removed

```
packages/my-app/.oxfmtrc.json: missing
```
