import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { parse as parseJsonc } from 'jsonc-parser';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { detectExistingEditors, selectEditors, writeEditorConfigs } from '../editor.js';

const tempDirs: string[] = [];

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-editor-config-'));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

describe('selectEditors', () => {
  it('prompts with editor config targets and supports multiple selections', async () => {
    const multiselectSpy = vi.spyOn(prompts, 'multiselect').mockResolvedValue(['vscode', 'zed']);

    await expect(
      selectEditors({
        interactive: true,
        onCancel: vi.fn(),
      }),
    ).resolves.toEqual(['vscode', 'zed']);

    expect(multiselectSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        message: expect.stringContaining('Which editors are you using?'),
        initialValues: ['vscode'],
        required: false,
        options: expect.arrayContaining([
          expect.objectContaining({
            label: 'VSCode',
            value: 'vscode',
            hint: '.vscode',
          }),
          expect.objectContaining({
            label: 'Zed',
            value: 'zed',
            hint: '.zed',
          }),
        ]),
      }),
    );
  });

  it('skips editor config selection when no editors are selected', async () => {
    vi.spyOn(prompts, 'multiselect').mockResolvedValue([]);

    await expect(
      selectEditors({
        interactive: true,
        onCancel: vi.fn(),
      }),
    ).resolves.toBeUndefined();
  });

  it('keeps explicit --editor selection as a single editor', async () => {
    await expect(
      selectEditors({
        interactive: false,
        editor: 'zed',
        onCancel: vi.fn(),
      }),
    ).resolves.toEqual(['zed']);
  });
});

describe('detectExistingEditors', () => {
  it('detects multiple existing editor config directories', () => {
    const projectRoot = createTempDir();
    fs.mkdirSync(path.join(projectRoot, '.vscode'), { recursive: true });
    fs.mkdirSync(path.join(projectRoot, '.zed'), { recursive: true });
    fs.writeFileSync(path.join(projectRoot, '.vscode', 'settings.json'), '{}');
    fs.writeFileSync(path.join(projectRoot, '.zed', 'settings.json'), '{}');

    expect(detectExistingEditors(projectRoot)).toEqual(['vscode', 'zed']);
  });

  it('returns undefined when no editor config files exist', () => {
    expect(detectExistingEditors(createTempDir())).toBeUndefined();
  });
});

describe('writeEditorConfigs', () => {
  it('writes vscode settings that align formatter config with vite.config.ts', async () => {
    const projectRoot = createTempDir();

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
    expect(settings['oxc.fmt.configPath']).toBe('./vite.config.ts');
    expect(settings['editor.formatOnSave']).toBe(true);
    expect(settings['npm.scriptRunner']).toBeUndefined();
    for (const lang of ['[javascript]', '[javascriptreact]', '[typescript]', '[typescriptreact]']) {
      expect(settings[lang]).toEqual({ 'editor.defaultFormatter': 'oxc.oxc-vscode' });
    }
  });

  it('includes additionalSettings in vscode settings.json when provided', async () => {
    const projectRoot = createTempDir();

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(settings['npm.scriptRunner']).toBe('vp');
    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
  });

  it('merges existing vscode JSONC settings (comments, trailing commas)', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  // JSONC comment
  "editor.formatOnSave": false,
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode",
  },
  "editor.codeActionsOnSave": {
    // preserve existing key
    "source.organizeImports": "explicit",
  },
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });

    const resultText = fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8');

    // Comments survive the merge
    expect(resultText).toContain('// JSONC comment');
    expect(resultText).toContain('// preserve existing key');

    const settings = parseJsonc(resultText) as Record<string, unknown>;

    // Existing key is preserved (merge never overwrites)
    expect(settings['editor.formatOnSave']).toBe(false);
    expect(settings['[typescript]']).toEqual({
      'editor.defaultFormatter': 'esbenp.prettier-vscode',
    });

    // New keys are added
    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
    expect(settings['oxc.fmt.configPath']).toBe('./vite.config.ts');
    expect(settings['npm.scriptRunner']).toBe('vp');
    for (const lang of ['[javascript]', '[javascriptreact]', '[typescriptreact]']) {
      expect(settings[lang]).toEqual({ 'editor.defaultFormatter': 'oxc.oxc-vscode' });
    }

    const codeActions = settings['editor.codeActionsOnSave'] as Record<string, unknown>;
    expect(codeActions['source.organizeImports']).toBe('explicit');
    expect(codeActions['source.fixAll.oxc']).toBe('explicit');
  });

  it('preserves a top-level comment before an existing setting (Vue Core style)', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  // Use the project's typescript version
  "typescript.tsdk": "node_modules/typescript/lib"
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const resultText = fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8');

    expect(resultText).toContain("// Use the project's typescript version");

    const settings = parseJsonc(resultText) as Record<string, unknown>;
    expect(settings['typescript.tsdk']).toBe('node_modules/typescript/lib');
    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
  });

  it('preserves a nested comment while adding source.fixAll.oxc', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  "editor.codeActionsOnSave": {
    // keep my organize imports
    "source.organizeImports": "explicit"
  }
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const resultText = fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8');

    expect(resultText).toContain('// keep my organize imports');

    const settings = parseJsonc(resultText) as Record<string, unknown>;
    const codeActions = settings['editor.codeActionsOnSave'] as Record<string, unknown>;
    expect(codeActions['source.organizeImports']).toBe('explicit');
    expect(codeActions['source.fixAll.oxc']).toBe('explicit');
  });

  it('never overwrites existing values during merge', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  "editor.formatOnSave": false,
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  }
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const settings = parseJsonc(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(settings['editor.formatOnSave']).toBe(false);
    expect(settings['[typescript]']).toEqual({
      'editor.defaultFormatter': 'esbenp.prettier-vscode',
    });
  });

  it('keeps trailing-comma JSONC valid after merge', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  "editor.codeActionsOnSave": {
    "source.organizeImports": "explicit",
  },
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const settings = parseJsonc(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
    const codeActions = settings['editor.codeActionsOnSave'] as Record<string, unknown>;
    expect(codeActions['source.organizeImports']).toBe('explicit');
    expect(codeActions['source.fixAll.oxc']).toBe('explicit');
  });

  it('appends extension recommendation without losing comments or duplicating', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'extensions.json'),
      `{
  "recommendations": [
    // keep my favorite extension
    "dbaeumer.vscode-eslint",
  ],
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });

    const resultText = fs.readFileSync(
      path.join(projectRoot, '.vscode', 'extensions.json'),
      'utf8',
    );

    expect(resultText).toContain('// keep my favorite extension');

    const extensions = parseJsonc(resultText) as { recommendations: string[] };
    expect(extensions.recommendations).toContain('dbaeumer.vscode-eslint');
    expect(
      extensions.recommendations.filter((r) => r === 'VoidZero.vite-plus-extension-pack'),
    ).toHaveLength(1);
  });

  it('is idempotent: a second merge makes no textual change', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      `{
  // JSONC comment
  "editor.formatOnSave": false,
}
`,
      'utf8',
    );

    const settingsPath = path.join(projectRoot, '.vscode', 'settings.json');

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });
    const afterFirst = fs.readFileSync(settingsPath, 'utf8');

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
    });
    const afterSecond = fs.readFileSync(settingsPath, 'utf8');

    expect(afterSecond).toBe(afterFirst);
  });

  it('preserves an existing Zed JSONC comment while adding nested settings', async () => {
    const projectRoot = createTempDir();

    const zedDir = path.join(projectRoot, '.zed');
    fs.mkdirSync(zedDir, { recursive: true });
    fs.writeFileSync(
      path.join(zedDir, 'settings.json'),
      `{
  // my zed settings
  "lsp": {
    "oxlint": {
      // keep this comment
      "initialization_options": {}
    }
  }
}
`,
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'zed',
      interactive: false,
      silent: true,
    });

    const resultText = fs.readFileSync(path.join(projectRoot, '.zed', 'settings.json'), 'utf8');

    expect(resultText).toContain('// my zed settings');
    expect(resultText).toContain('// keep this comment');

    const settings = parseJsonc(resultText) as {
      lsp?: { oxfmt?: { initialization_options?: { settings?: Record<string, unknown> } } };
    };
    expect(settings.lsp?.oxfmt?.initialization_options?.settings?.['fmt.configPath']).toBe(
      './vite.config.ts',
    );
  });

  it('does not apply extraVsCodeSettings to zed editor', async () => {
    const projectRoot = createTempDir();

    await writeEditorConfigs({
      projectRoot,
      editorId: 'zed',
      interactive: false,
      silent: true,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.zed', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(settings['npm.scriptRunner']).toBeUndefined();
  });

  it('preserves existing npm.scriptRunner during merge with extraVsCodeSettings', async () => {
    const projectRoot = createTempDir();

    const vscodeDir = path.join(projectRoot, '.vscode');
    fs.mkdirSync(vscodeDir, { recursive: true });
    fs.writeFileSync(
      path.join(vscodeDir, 'settings.json'),
      JSON.stringify({ 'npm.scriptRunner': 'npm' }),
      'utf8',
    );

    await writeEditorConfigs({
      projectRoot,
      editorId: 'vscode',
      interactive: false,
      silent: true,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    // deepMerge preserves existing keys — 'npm' is not overwritten by 'vp'
    expect(settings['npm.scriptRunner']).toBe('npm');
  });

  it('writes zed settings that align formatter config with vite.config.ts', async () => {
    const projectRoot = createTempDir();

    await writeEditorConfigs({
      projectRoot,
      editorId: 'zed',
      interactive: false,
      silent: true,
    });

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.zed', 'settings.json'), 'utf8'),
    ) as {
      lsp?: {
        oxfmt?: {
          initialization_options?: {
            settings?: {
              'fmt.configPath'?: string;
            };
          };
        };
      };
    };

    expect(settings.lsp?.oxfmt?.initialization_options?.settings?.['fmt.configPath']).toBe(
      './vite.config.ts',
    );
  });

  it('writes multiple editor configs in one call', async () => {
    const projectRoot = createTempDir();

    await writeEditorConfigs({
      projectRoot,
      editorId: ['vscode', 'zed'],
      interactive: false,
      silent: true,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });

    const vscodeSettings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;
    const vscodeExtensions = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'extensions.json'), 'utf8'),
    ) as Record<string, unknown>;
    const zedSettings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.zed', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    expect(vscodeSettings['npm.scriptRunner']).toBe('vp');
    expect(vscodeExtensions.recommendations).toContain('VoidZero.vite-plus-extension-pack');
    expect(zedSettings['npm.scriptRunner']).toBeUndefined();
    expect(zedSettings.lsp).toBeDefined();
  });
});
