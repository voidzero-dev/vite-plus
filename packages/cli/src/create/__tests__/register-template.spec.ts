import fs from 'node:fs';
import path from 'node:path';

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest';

import { resolveViteConfig } from '../../resolve-vite-config.js';
import type { CreateTemplateEntry } from '../org-manifest.js';
import { registerLocalTemplate } from '../register-template.js';

/**
 * Walk up from this test file to the package/repo whose `node_modules`
 * contains `vite-plus`. Temp workspaces are created *inside* that
 * `node_modules` so the `import { defineConfig } from 'vite-plus'` lines
 * the helper writes (and that `resolveViteConfig` evaluates) resolve.
 * A detached `os.tmpdir()` cannot resolve `vite-plus`.
 */
function findVitePlusRoot(): string {
  let dir = import.meta.dirname;
  while (dir !== path.dirname(dir)) {
    if (fs.existsSync(path.join(dir, 'node_modules', 'vite-plus'))) {
      return dir;
    }
    dir = path.dirname(dir);
  }
  throw new Error('could not locate a node_modules with vite-plus');
}

const TMP_PARENT = path.join(findVitePlusRoot(), 'node_modules', '.vp-register-template-tests');

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
    fs.mkdirSync(TMP_PARENT, { recursive: true });
    workspaceRoot = fs.mkdtempSync(path.join(TMP_PARENT, 'ws-'));
  });

  afterEach(() => {
    fs.rmSync(workspaceRoot, { recursive: true, force: true });
  });

  afterAll(() => {
    fs.rmSync(TMP_PARENT, { recursive: true, force: true });
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
});
