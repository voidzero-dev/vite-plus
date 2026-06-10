import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { resolveViteConfig } from '../../resolve-vite-config.js';
import type { CreateTemplateEntry } from '../org-manifest.js';
import { registerLocalTemplate } from '../register-template.js';

const ENTRY_A: CreateTemplateEntry = {
  name: 'my-generator',
  description: 'A local generator',
  template: './templates/my-generator',
};

const ENTRY_B: CreateTemplateEntry = {
  name: 'other-generator',
  description: 'Another local generator',
  template: './templates/other-generator',
};

describe('registerLocalTemplate', () => {
  let workspaceRoot: string;

  beforeEach(() => {
    // Self-contained workspace in the OS temp dir with a stubbed `vite-plus`
    // so the `import { defineConfig } from 'vite-plus'` lines the helper
    // writes (and that `resolveViteConfig` evaluates) resolve. Using a shared
    // fixture dir inside the repo's real node_modules would race concurrent
    // test runs and leave junk behind on a killed run.
    workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-register-template-'));
    fs.writeFileSync(path.join(workspaceRoot, 'package.json'), '{"type":"module"}');
    const stubDir = path.join(workspaceRoot, 'node_modules', 'vite-plus');
    fs.mkdirSync(stubDir, { recursive: true });
    fs.writeFileSync(path.join(stubDir, 'package.json'), '{"type":"module","main":"index.js"}');
    fs.writeFileSync(path.join(stubDir, 'index.js'), 'export const defineConfig = (c) => c;\n');
  });

  afterEach(() => {
    fs.rmSync(workspaceRoot, { recursive: true, force: true });
    vi.restoreAllMocks();
  });

  function writeViteConfig(body: string): void {
    fs.writeFileSync(
      path.join(workspaceRoot, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig(${body});\n`,
    );
  }

  async function readCreate(): Promise<{
    defaultTemplate?: string;
    templates?: CreateTemplateEntry[];
  }> {
    const config = (await resolveViteConfig(workspaceRoot)) as {
      create?: { defaultTemplate?: string; templates?: CreateTemplateEntry[] };
    };
    return config.create ?? {};
  }

  it('creates a vite.config.ts with create.templates when none exists', async () => {
    expect(fs.existsSync(path.join(workspaceRoot, 'vite.config.ts'))).toBe(false);

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    expect(fs.existsSync(path.join(workspaceRoot, 'vite.config.ts'))).toBe(true);
    const create = await readCreate();
    expect(create.defaultTemplate).toBeUndefined();
    expect(create.templates).toEqual([ENTRY_A]);
  });

  it('targets an existing vite.config.mts instead of creating a stray vite.config.ts', async () => {
    // A monorepo whose only config is a .mts (or .cts/.cjs) file must be the
    // registration target. Missing those extensions would create a new
    // vite.config.ts and write to it, leaving the real config untouched.
    fs.writeFileSync(
      path.join(workspaceRoot, 'vite.config.mts'),
      `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({ create: { defaultTemplate: '@your-org' } });\n`,
    );

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    expect(fs.existsSync(path.join(workspaceRoot, 'vite.config.ts'))).toBe(false);
    const create = await readCreate();
    expect(create.defaultTemplate).toBe('@your-org');
    expect(create.templates).toEqual([ENTRY_A]);
  });

  it('appends templates while preserving an existing defaultTemplate', async () => {
    writeViteConfig("{ create: { defaultTemplate: '@your-org' } }");

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    const create = await readCreate();
    expect(create.defaultTemplate).toBe('@your-org');
    expect(create.templates).toEqual([ENTRY_A]);
  });

  it('is a no-op when an entry with the same name already exists', async () => {
    writeViteConfig(
      `{ create: { templates: [{ name: '${ENTRY_A.name}', description: 'pre-existing', template: './pre-existing' }] } }`,
    );
    const before = fs.readFileSync(path.join(workspaceRoot, 'vite.config.ts'), 'utf8');

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    const after = fs.readFileSync(path.join(workspaceRoot, 'vite.config.ts'), 'utf8');
    expect(after).toBe(before);
    const create = await readCreate();
    // The pre-existing entry (with its original description) is untouched.
    expect(create.templates).toEqual([
      { name: ENTRY_A.name, description: 'pre-existing', template: './pre-existing' },
    ]);
  });

  it('appends a second, different entry after the first', async () => {
    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);
    await registerLocalTemplate(workspaceRoot, ENTRY_B, true);

    const create = await readCreate();
    expect(create.templates).toEqual([ENTRY_A, ENTRY_B]);
  });

  it('preserves defaultTemplate and prior templates across appends', async () => {
    writeViteConfig("{ create: { defaultTemplate: '@your-org' } }");

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);
    await registerLocalTemplate(workspaceRoot, ENTRY_B, true);

    const create = await readCreate();
    expect(create.defaultTemplate).toBe('@your-org');
    expect(create.templates).toEqual([ENTRY_A, ENTRY_B]);
  });

  it('does not clobber a pre-existing .vite-plus-create-register.json in the workspace', async () => {
    // The temp file used for the merge must not collide with a user file of a
    // fixed name in the workspace root.
    const sentinel = path.join(workspaceRoot, '.vite-plus-create-register.json');
    fs.writeFileSync(sentinel, '{"keep":true}');

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    expect(fs.existsSync(sentinel)).toBe(true);
    expect(JSON.parse(fs.readFileSync(sentinel, 'utf8'))).toEqual({ keep: true });
  });

  it('aborts without clobbering when the existing config cannot be evaluated', async () => {
    // The config exists with a real create block but fails to evaluate (missing
    // import). Treating that as empty would replace the block with only the new
    // entry, dropping defaultTemplate and prior templates. It must abort.
    const configPath = path.join(workspaceRoot, 'vite.config.ts');
    const original = `import { defineConfig } from 'vite-plus';\nimport 'vite-plus-nonexistent-module-xyz';\n\nexport default defineConfig({ create: { defaultTemplate: '@your-org', templates: [{ name: 'pre', description: 'p', template: './pre' }] } });\n`;
    fs.writeFileSync(configPath, original);

    await expect(registerLocalTemplate(workspaceRoot, ENTRY_A, true)).rejects.toThrow();
    expect(fs.readFileSync(configPath, 'utf8')).toBe(original);
  });

  it('preserves unrelated sibling config when adding a create block', async () => {
    writeViteConfig('{ run: { cache: true } }');

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    const config = (await resolveViteConfig(workspaceRoot)) as {
      run?: { cache?: boolean };
      create?: { templates?: CreateTemplateEntry[] };
    };
    expect(config.run?.cache).toBe(true);
    expect(config.create?.templates).toEqual([ENTRY_A]);
  });

  it('warns when a same-name entry already points at a different template', async () => {
    const warnSpy = vi.spyOn(prompts.log, 'warn').mockImplementation(() => {});
    writeViteConfig(
      `{ create: { templates: [{ name: '${ENTRY_A.name}', description: 'pre', template: './old-path' }] } }`,
    );

    const result = await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    // Still a no-op, but the stale entry is called out instead of silently
    // shadowing the new generator.
    expect(result).toBeUndefined();
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('./old-path'));
  });

  it('throws on an unsupported config shape instead of writing nothing', async () => {
    // `export default someVar` has no direct config object the upsert can
    // edit. Reporting success while writing nothing would silently leave the
    // generator unregistered.
    const configPath = path.join(workspaceRoot, 'vite.config.ts');
    const original = 'const config = { create: { templates: [] } };\n\nexport default config;\n';
    fs.writeFileSync(configPath, original);

    await expect(registerLocalTemplate(workspaceRoot, ENTRY_A, true)).rejects.toThrow(
      /supported config object/,
    );
    expect(fs.readFileSync(configPath, 'utf8')).toBe(original);
  });

  it('replaces a shorthand `create` property with the recomputed block', async () => {
    // `defineConfig({ create })` — a prepended duplicate key would be
    // overridden by the shorthand at runtime; the shorthand itself must be
    // replaced so the registered entry is live.
    fs.writeFileSync(
      path.join(workspaceRoot, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';\n\nconst create = { defaultTemplate: '@your-org' };\n\nexport default defineConfig({ create });\n`,
    );

    await registerLocalTemplate(workspaceRoot, ENTRY_A, true);

    const create = await readCreate();
    expect(create.defaultTemplate).toBe('@your-org');
    expect(create.templates).toEqual([ENTRY_A]);
  });
});
