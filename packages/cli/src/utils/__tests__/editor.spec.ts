import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
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

    const settings = JSON.parse(
      fs.readFileSync(path.join(projectRoot, '.vscode', 'settings.json'), 'utf8'),
    ) as Record<string, unknown>;

    // Existing key is preserved (merge never overwrites)
    expect(settings['editor.formatOnSave']).toBe(false);

    // New keys are added
    expect(settings['editor.defaultFormatter']).toBe('oxc.oxc-vscode');
    expect(settings['oxc.fmt.configPath']).toBe('./vite.config.ts');
    expect(settings['npm.scriptRunner']).toBe('vp');

    const codeActions = settings['editor.codeActionsOnSave'] as Record<string, unknown>;
    expect(codeActions['source.organizeImports']).toBe('explicit');
    expect(codeActions['source.fixAll.oxc']).toBe('explicit');
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
              configPath?: string;
            };
          };
        };
      };
    };

    expect(settings.lsp?.oxfmt?.initialization_options?.settings?.configPath).toBe(
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
