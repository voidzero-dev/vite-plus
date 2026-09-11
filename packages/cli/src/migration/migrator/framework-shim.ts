import fs from 'node:fs';
import path from 'node:path';

import { readJsonFile } from '../../utils/json.ts';
import { type MigrationReport } from '../report.ts';

// .svelte files are handled by @sveltejs/vite-plugin-svelte (transpilation)
// and svelte-check / Svelte Language Server (type checking).
// Module resolution for `.svelte` imports is typically set up by the
// project template (e.g. src/vite-env.d.ts in Vite svelte-ts, or
// auto-generated tsconfig in SvelteKit) rather than this file.
// https://svelte.dev/docs/svelte/typescript
export type Framework = 'vue' | 'astro';

const FRAMEWORK_SHIMS: Record<Framework, string> = {
  // https://vuejs.org/guide/typescript/overview#volar-takeover-mode
  vue: [
    "declare module '*.vue' {",
    "  import type { DefineComponent } from 'vue';",
    '  const component: DefineComponent<{}, {}, unknown>;',
    '  export default component;',
    '}',
  ].join('\n'),
  // astro/client is the pre-v4.14 form; v4.14+ prefers `/// <reference path="../.astro/types.d.ts" />`
  // but .astro/types.d.ts is generated at build time and may not exist yet after migration.
  // astro/client remains valid and is still used in official Astro integrations.
  // https://docs.astro.build/en/guides/typescript/#extending-global-types
  astro: '/// <reference types="astro/client" />',
};

export function detectFramework(projectPath: string): Framework[] {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return [];
  }
  const pkg = readJsonFile(packageJsonPath) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  };
  const allDeps = { ...pkg.dependencies, ...pkg.devDependencies };
  return (['vue', 'astro'] as const).filter((framework) => !!allDeps[framework]);
}

function getEnvDtsPath(projectPath: string): string {
  const srcEnvDts = path.join(projectPath, 'src', 'env.d.ts');
  const rootEnvDts = path.join(projectPath, 'env.d.ts');
  for (const candidate of [srcEnvDts, rootEnvDts]) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return fs.existsSync(path.join(projectPath, 'src')) ? srcEnvDts : rootEnvDts;
}

export function hasFrameworkShim(projectPath: string, framework: Framework): boolean {
  const dirsToScan = [projectPath, path.join(projectPath, 'src')];
  for (const dir of dirsToScan) {
    if (!fs.existsSync(dir)) {
      continue;
    }
    let entries: string[];
    try {
      entries = fs.readdirSync(dir);
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (!entry.endsWith('.d.ts')) {
        continue;
      }
      const content = fs.readFileSync(path.join(dir, entry), 'utf-8');
      if (framework === 'astro') {
        if (content.includes('astro/client')) {
          return true;
        }
      } else if (content.includes(`'*.${framework}'`) || content.includes(`"*.${framework}"`)) {
        return true;
      }
    }
  }
  return false;
}

export function addFrameworkShim(
  projectPath: string,
  framework: Framework,
  report?: MigrationReport,
): void {
  const envDtsPath = getEnvDtsPath(projectPath);
  const shim = FRAMEWORK_SHIMS[framework];
  if (fs.existsSync(envDtsPath)) {
    const existing = fs.readFileSync(envDtsPath, 'utf-8');
    fs.writeFileSync(envDtsPath, `${existing.trimEnd()}\n\n${shim}\n`, 'utf-8');
  } else {
    fs.mkdirSync(path.dirname(envDtsPath), { recursive: true });
    fs.writeFileSync(envDtsPath, `${shim}\n`, 'utf-8');
  }
  if (report) {
    report.frameworkShimAdded = true;
  }
}
