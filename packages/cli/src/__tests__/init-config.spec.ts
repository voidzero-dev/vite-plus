import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';
import { vi } from 'vitest';

import { applyToolInitConfigToViteConfig, inspectInitCommand } from '../init-config.js';

const tempDirs: string[] = [];

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-init-config-'));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  vi.clearAllMocks();
});

describe('applyToolInitConfigToViteConfig', () => {
  it('returns false for non-init command invocations', async () => {
    const projectPath = createTempDir();
    await expect(
      applyToolInitConfigToViteConfig('lint', ['src/index.ts'], projectPath),
    ).resolves.toEqual({ handled: false });
  });

  it('creates vite.config.ts and writes lint config with options for vp lint --init', async () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, '.oxlintrc.json'),
      JSON.stringify(
        {
          rules: {
            eqeqeq: 'warn',
          },
        },
        null,
        2,
      ),
    );

    const result = await applyToolInitConfigToViteConfig('lint', ['--init'], projectPath);
    expect(result.handled).toBe(true);
    expect(result.action).toBe('added');

    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    expect(fs.existsSync(viteConfigPath)).toBe(true);
    const content = fs.readFileSync(viteConfigPath, 'utf8');
    expect(content).toContain('import { defineConfig } from');
    expect(content).toContain('vite-plus');
    expect(content).toContain('typeAware');
    expect(content).toContain('typeCheck');
    expect(fs.existsSync(path.join(projectPath, '.oxlintrc.json'))).toBe(false);
  });

  it('ignores generated lint init defaults and still writes lint with options', async () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, '.oxlintrc.json'),
      JSON.stringify(
        {
          plugins: null,
          categories: {},
          rules: {},
          settings: {
            'jsx-a11y': {
              polymorphicPropName: null,
              components: {},
              attributes: {},
            },
            next: {
              rootDir: [],
            },
            react: {
              formComponents: [],
              linkComponents: [],
              version: null,
              componentWrapperFunctions: [],
            },
            jsdoc: {
              ignorePrivate: false,
              ignoreInternal: false,
              ignoreReplacesDocs: true,
              overrideReplacesDocs: true,
              augmentsExtendsReplacesDocs: false,
              implementsReplacesDocs: false,
              exemptDestructuredRootsFromChecks: false,
              tagNamePreference: {},
            },
            vitest: {
              typecheck: false,
            },
          },
          env: {
            builtin: true,
          },
          globals: {},
          ignorePatterns: [],
        },
        null,
        2,
      ),
    );

    const result = await applyToolInitConfigToViteConfig('lint', ['--init'], projectPath);
    expect(result.handled).toBe(true);
    expect(result.action).toBe('added');

    const content = fs.readFileSync(path.join(projectPath, 'vite.config.ts'), 'utf8');
    expect(content).toContain('typeAware');
    expect(content).toContain('typeCheck');
    expect(content).not.toContain('jsx-a11y');
    expect(content).not.toContain('ignorePatterns');
  });

  it('inlines fmt migrate output into existing vite config', async () => {
    const projectPath = createTempDir();
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});
`,
    );
    fs.writeFileSync(path.join(projectPath, '.oxfmtrc.json'), '{\n  "semi": true\n}\n');

    const result = await applyToolInitConfigToViteConfig(
      'fmt',
      ['--migrate=prettier'],
      projectPath,
    );
    expect(result.handled).toBe(true);
    expect(result.action).toBe('added');

    const content = fs.readFileSync(viteConfigPath, 'utf8');
    expect(content).toContain('fmt:');
    expect(content).toContain('semi');
    expect(fs.existsSync(path.join(projectPath, '.oxfmtrc.json'))).toBe(false);
  });

  it('uses explicit --config path when provided', async () => {
    const projectPath = createTempDir();
    const customConfigPath = path.join(projectPath, 'custom-oxfmt.json');
    fs.writeFileSync(customConfigPath, '{\n  "tabWidth": 4\n}\n');

    const result = await applyToolInitConfigToViteConfig(
      'fmt',
      ['--init', '--config', 'custom-oxfmt.json'],
      projectPath,
    );
    expect(result.handled).toBe(true);
    expect(result.action).toBe('added');

    const content = fs.readFileSync(path.join(projectPath, 'vite.config.ts'), 'utf8');
    expect(content).toContain('fmt:');
    expect(content).toContain('tabWidth');
    expect(fs.existsSync(customConfigPath)).toBe(false);
  });

  it('removes generated file when key already exists', async () => {
    const projectPath = createTempDir();
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    const existing = `import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: {},
  },
});
`;
    fs.writeFileSync(viteConfigPath, existing);
    fs.writeFileSync(
      path.join(projectPath, '.oxlintrc.json'),
      '{\n  "rules": { "no-console": "warn" }\n}\n',
    );

    const result = await applyToolInitConfigToViteConfig('lint', ['--init'], projectPath);
    expect(result.handled).toBe(true);
    expect(result.action).toBe('skipped-existing');
    expect(fs.readFileSync(viteConfigPath, 'utf8')).toBe(existing);
    expect(fs.existsSync(path.join(projectPath, '.oxlintrc.json'))).toBe(false);
  });

  it('detects existing init key before running native init', () => {
    const projectPath = createTempDir();
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
});
`,
    );
    const inspection = inspectInitCommand('fmt', ['--init'], projectPath);
    expect(inspection.handled).toBe(true);
    expect(inspection.configKey).toBe('fmt');
    expect(inspection.hasExistingConfigKey).toBe(true);
    expect(inspection.existingViteConfigPath).toBe(viteConfigPath);
  });
});
