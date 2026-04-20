import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { writeEditorConfigs } from '../editor.js';

const tempDirs: string[] = [];

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-editor-config-'));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
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
});
