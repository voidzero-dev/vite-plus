import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  deriveDefaultPackageName,
  ensureDefaultGitignoreEntries,
  ensureGitignoreVsCodeEditorConfigs,
  formatTargetDir,
  getProjectDirFromPackageName,
  normalizeEditorOption,
  renameFiles,
  shouldConfigureEditorsForCreate,
} from '../utils.js';

describe('getProjectDirFromPackageName', () => {
  it('should get project dir from package name', () => {
    expect(getProjectDirFromPackageName('@my/package')).toBe('package');
    expect(getProjectDirFromPackageName('my-package')).toBe('my-package');
  });
});

describe('editor configuration policy', () => {
  it('normalizes repeated editor options to a single editor value', () => {
    expect(normalizeEditorOption('vscode')).toBe('vscode');
    expect(normalizeEditorOption(['vscode', 'zed'])).toBe('zed');
    expect(normalizeEditorOption(['vscode', false])).toBe(false);
    expect(normalizeEditorOption([undefined, 'vscode'])).toBe('vscode');
  });

  it('allows automatic editor configuration outside existing monorepos', () => {
    expect(shouldConfigureEditorsForCreate({ isMonorepo: false, editor: undefined })).toBe(true);
  });

  it('skips automatic editor configuration inside existing monorepos', () => {
    expect(shouldConfigureEditorsForCreate({ isMonorepo: true, editor: undefined })).toBe(false);
  });

  it('allows explicit editor opt-in inside existing monorepos', () => {
    expect(shouldConfigureEditorsForCreate({ isMonorepo: true, editor: 'vscode' })).toBe(true);
    expect(shouldConfigureEditorsForCreate({ isMonorepo: true, editor: '   ' })).toBe(false);
  });

  it('keeps --no-editor as an explicit opt-out in every workspace mode', () => {
    expect(shouldConfigureEditorsForCreate({ isMonorepo: false, editor: false })).toBe(false);
    expect(shouldConfigureEditorsForCreate({ isMonorepo: true, editor: false })).toBe(false);
  });
});

describe('formatTargetDir', () => {
  it('should format "." as current directory with empty package name', () => {
    expect(formatTargetDir('.')).toEqual({
      directory: '.',
      packageName: '',
    });
  });

  it('should format "./" as current directory with empty package name', () => {
    expect(formatTargetDir('./')).toEqual({
      directory: '.',
      packageName: '',
    });
  });

  it('should format target dir with invalid input', () => {
    expect(formatTargetDir('/foo/bar')).matchSnapshot();
    expect(formatTargetDir('@scope/')).matchSnapshot();
    expect(formatTargetDir('../../foo/bar')).matchSnapshot();
  });

  // Should work on all platforms (including Windows) - directory must always use forward slashes
  it('should format target dir with valid input', () => {
    expect(formatTargetDir('./my-package')).matchSnapshot();
    expect(formatTargetDir('my-package')).matchSnapshot();
    expect(formatTargetDir('@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/sub-package')).matchSnapshot();
  });

  // Regression test for https://github.com/voidzero-dev/vite-plus/issues/938
  // On Windows, path.join/normalize produce backslashes which break when passed as CLI args.
  // Nested paths are the critical cases since they involve path separators.
  it('should always use forward slashes in directory (issue #938)', () => {
    expect(formatTargetDir('foo/@my-scope/my-package').directory).toBe('foo/my-package');
    expect(formatTargetDir('./foo/bar/@scope/my-package').directory).toBe('foo/bar/my-package');
    expect(formatTargetDir('./foo/bar/@scope/my-package/sub-package').directory).toBe(
      'foo/bar/@scope/my-package/sub-package',
    );
  });

  it('should format target dir with invalid package name', () => {
    expect(formatTargetDir('my-package@').error).matchSnapshot();
    expect(formatTargetDir('my-package@1.0.0').error).matchSnapshot();
  });
});

describe('deriveDefaultPackageName', () => {
  it('should derive package name from directory basename', () => {
    expect(deriveDefaultPackageName('/home/user/my-app', undefined, 'fallback')).toBe('my-app');
  });

  it('should derive scoped package name when scope is provided', () => {
    expect(deriveDefaultPackageName('/home/user/my-app', '@my-scope', 'fallback')).toBe(
      '@my-scope/my-app',
    );
  });

  it('should fallback to random name when directory name is invalid', () => {
    const result = deriveDefaultPackageName('/home/user/.hidden', undefined, 'vite-plus-app');
    // directory name starts with '.', so a random name is generated instead
    expect(result).not.toBe('.hidden');
    expect(result.length).toBeGreaterThan(0);
  });

  it('should fallback when directory is filesystem root', () => {
    const result = deriveDefaultPackageName('/', undefined, 'vite-plus-app');
    // basename of '/' is empty, so a random name is generated
    expect(result.length).toBeGreaterThan(0);
  });
});

describe('ensureDefaultGitignoreEntries', () => {
  let projectDir: string;

  beforeEach(() => {
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-gitignore-'));
  });

  afterEach(() => {
    fs.rmSync(projectDir, { recursive: true, force: true });
  });

  function gitignore(): string {
    return fs.readFileSync(path.join(projectDir, '.gitignore'), 'utf-8');
  }

  const dotenvBlock = '# dotenv environment variable files\n.env\n.env.*\n!.env.example\n';

  it('creates a fresh `.gitignore` with default entries when none exists', () => {
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`node_modules\n\n${dotenvBlock}`);
  });

  it('appends default entries to an existing `.gitignore` that omits them', () => {
    fs.writeFileSync(path.join(projectDir, '.gitignore'), 'dist\n*.log\n');
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`dist\n*.log\nnode_modules\n\n${dotenvBlock}`);
  });

  it('terminates the last line first when the existing file lacks a trailing newline', () => {
    fs.writeFileSync(path.join(projectDir, '.gitignore'), 'dist');
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`dist\nnode_modules\n\n${dotenvBlock}`);
  });

  it('appends dotenv entries when `node_modules` already appears as a standalone line', () => {
    const existing = '# Logs\n*.log\nnode_modules\ndist\n';
    fs.writeFileSync(path.join(projectDir, '.gitignore'), existing);
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`${existing}\n${dotenvBlock}`);
  });

  it('treats `node_modules/` (with trailing slash) as a match', () => {
    const existing = 'node_modules/\ndist\n';
    fs.writeFileSync(path.join(projectDir, '.gitignore'), existing);
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`${existing}\n${dotenvBlock}`);
  });

  it('handles CRLF line endings without re-appending existing defaults', () => {
    const existing = `node_modules\r\ndist\r\n${dotenvBlock.replaceAll('\n', '\r\n')}`;
    fs.writeFileSync(path.join(projectDir, '.gitignore'), existing);
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(existing);
  });

  it('does not consider a `node_modules/sub` subpath as already excluded', () => {
    fs.writeFileSync(path.join(projectDir, '.gitignore'), 'node_modules/sub\n');
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`node_modules/sub\nnode_modules\n\n${dotenvBlock}`);
  });

  it('does not match `!node_modules` (an explicit un-ignore override)', () => {
    fs.writeFileSync(path.join(projectDir, '.gitignore'), '!node_modules\n');
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(`!node_modules\nnode_modules\n\n${dotenvBlock}`);
  });

  it('adds only missing dotenv entries', () => {
    fs.writeFileSync(
      path.join(projectDir, '.gitignore'),
      '# dotenv environment variable files\n.env\nnode_modules\n',
    );
    ensureDefaultGitignoreEntries(projectDir);
    expect(gitignore()).toBe(
      '# dotenv environment variable files\n.env\nnode_modules\n.env.*\n!.env.example\n',
    );
  });
});

describe('ensureGitignoreVsCodeEditorConfigs', () => {
  let projectDir: string;

  beforeEach(() => {
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-vscode-gitignore-'));
  });

  afterEach(() => {
    fs.rmSync(projectDir, { recursive: true, force: true });
  });

  function gitignore(): string {
    return fs.readFileSync(path.join(projectDir, '.gitignore'), 'utf-8');
  }

  function writeGitignore(content: string): void {
    fs.writeFileSync(path.join(projectDir, '.gitignore'), content);
  }

  function writeVsCodeSettings(): void {
    fs.mkdirSync(path.join(projectDir, '.vscode'), { recursive: true });
    fs.writeFileSync(path.join(projectDir, '.vscode', 'settings.json'), '{}\n');
  }

  const vscodeUnignoreBlock = '!.vscode/\n!.vscode/settings.json\n!.vscode/extensions.json\n';

  it('unignores VS Code settings when `.vscode/*` is ignored', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n!.vscode/extensions.json\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/*\n!.vscode/extensions.json\n${vscodeUnignoreBlock}`);
  });

  it('unignores generated VS Code config files for root-anchored contents ignores', () => {
    writeVsCodeSettings();
    writeGitignore('/.vscode/*\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`/.vscode/*\n${vscodeUnignoreBlock}`);
  });

  it('appends VS Code directory and config unignores for directory-level VS Code ignores', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/\n${vscodeUnignoreBlock}`);
  });

  it('appends VS Code directory and config unignores for root-anchored directory-level VS Code ignores', () => {
    writeVsCodeSettings();
    writeGitignore('/.vscode\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`/.vscode\n${vscodeUnignoreBlock}`);
  });

  it('appends VS Code config unignores after explicit VS Code settings ignores', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n.vscode/settings.json\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/*\n.vscode/settings.json\n${vscodeUnignoreBlock}`);
  });

  it('appends VS Code config unignores after explicit VS Code extensions ignores', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n/.vscode/extensions.json\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/*\n/.vscode/extensions.json\n${vscodeUnignoreBlock}`);
  });

  it('appends VS Code config unignores when all generated config files are explicitly ignored', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n.vscode/settings.json\n.vscode/extensions.json\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(
      `.vscode/*\n.vscode/settings.json\n.vscode/extensions.json\n${vscodeUnignoreBlock}`,
    );
  });

  it('appends the full block when settings are already unignored without the EOF block', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n!.vscode/settings.json\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/*\n!.vscode/settings.json\n${vscodeUnignoreBlock}`);
  });

  it('re-appends the full block when later ignore rules override generated config unignores', () => {
    writeVsCodeSettings();
    writeGitignore('!.vscode/settings.json\n!.vscode/extensions.json\n.vscode/*\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(
      `!.vscode/settings.json\n!.vscode/extensions.json\n.vscode/*\n${vscodeUnignoreBlock}`,
    );
  });

  it('appends VS Code config unignores even without a broad VS Code ignore', () => {
    writeVsCodeSettings();
    writeGitignore('dist\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`dist\n${vscodeUnignoreBlock}`);
  });

  it('does not create `.gitignore` when none exists', () => {
    writeVsCodeSettings();
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(fs.existsSync(path.join(projectDir, '.gitignore'))).toBe(false);
  });

  it('does not change `.gitignore` when VS Code settings do not exist', () => {
    const existing = '.vscode/*\n';
    writeGitignore(existing);
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(existing);
  });

  it('terminates the last line before appending VS Code config unignores', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(`.vscode/*\n${vscodeUnignoreBlock}`);
  });

  it('is idempotent', () => {
    writeVsCodeSettings();
    writeGitignore('.vscode/*\n');
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    const afterFirstRun = gitignore();
    ensureGitignoreVsCodeEditorConfigs(projectDir);
    expect(gitignore()).toBe(afterFirstRun);
  });
});

describe('renameFiles', () => {
  let projectDir: string;

  beforeEach(() => {
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-rename-'));
  });

  afterEach(() => {
    fs.rmSync(projectDir, { recursive: true, force: true });
  });

  function write(name: string, content: string): void {
    fs.writeFileSync(path.join(projectDir, name), content);
  }

  function read(name: string): string {
    return fs.readFileSync(path.join(projectDir, name), 'utf-8');
  }

  function exists(name: string): boolean {
    return fs.existsSync(path.join(projectDir, name));
  }

  it('renames `_gitignore` to `.gitignore`', () => {
    write('_gitignore', 'node_modules\n');
    renameFiles(projectDir);
    expect(exists('_gitignore')).toBe(false);
    expect(read('.gitignore')).toBe('node_modules\n');
  });

  it('renames `_npmrc` and `_yarnrc.yml`', () => {
    write('_npmrc', 'auto-install-peers=true\n');
    write('_yarnrc.yml', 'nodeLinker: node-modules\n');
    renameFiles(projectDir);
    expect(exists('_npmrc')).toBe(false);
    expect(exists('_yarnrc.yml')).toBe(false);
    expect(read('.npmrc')).toBe('auto-install-peers=true\n');
    expect(read('.yarnrc.yml')).toBe('nodeLinker: node-modules\n');
  });

  it('is a no-op when no source files exist', () => {
    expect(() => renameFiles(projectDir)).not.toThrow();
    expect(fs.readdirSync(projectDir)).toEqual([]);
  });

  it('leaves unmapped underscore files untouched', () => {
    write('_foo', 'bar\n');
    renameFiles(projectDir);
    expect(read('_foo')).toBe('bar\n');
  });
});
