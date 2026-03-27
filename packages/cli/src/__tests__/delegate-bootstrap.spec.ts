import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { getDelegatedBinPath, resolveLocalVitePlusBin } from '../delegate-bootstrap.js';

const tempDirs: string[] = [];
const bootstrapEnv = 'VITE_PLUS_DELEGATE_BOOTSTRAP_ACTIVE';

function createTempDir() {
  const dir = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vp-delegate-bootstrap-')));
  tempDirs.push(dir);
  return dir;
}

function createLocalVitePlus(projectDir: string, binContents = 'export {};\n') {
  const packageDir = path.join(projectDir, 'node_modules', 'vite-plus');
  fs.mkdirSync(path.join(packageDir, 'dist'), { recursive: true });
  fs.writeFileSync(path.join(packageDir, 'package.json'), '{"name":"vite-plus"}\n');
  fs.writeFileSync(path.join(packageDir, 'dist', 'bin.js'), binContents);
  return path.join(packageDir, 'dist', 'bin.js');
}

afterEach(() => {
  delete process.env[bootstrapEnv];
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

describe('delegate bootstrap', () => {
  it('resolves the local vite-plus bin from the project context', () => {
    const projectDir = createTempDir();
    const globalDir = createTempDir();
    const globalBinPath = path.join(globalDir, 'bin.js');
    fs.writeFileSync(globalBinPath, 'export {};\n');

    const localBinPath = createLocalVitePlus(projectDir);

    expect(resolveLocalVitePlusBin(projectDir, globalBinPath)).toBe(localBinPath);
    expect(getDelegatedBinPath(projectDir, globalBinPath)).toBe(localBinPath);
  });

  it('falls back to the global bin when no local vite-plus is installed', () => {
    const projectDir = createTempDir();
    const globalDir = createTempDir();
    const globalBinPath = path.join(globalDir, 'bin.js');
    fs.writeFileSync(globalBinPath, 'export {};\n');

    expect(resolveLocalVitePlusBin(projectDir, globalBinPath)).toBeNull();
    expect(getDelegatedBinPath(projectDir, globalBinPath)).toBe(globalBinPath);
  });

  it('does not recurse once the bootstrap is already active', () => {
    const projectDir = createTempDir();
    const globalDir = createTempDir();
    const globalBinPath = path.join(globalDir, 'bin.js');
    fs.writeFileSync(globalBinPath, 'export {};\n');
    createLocalVitePlus(projectDir);

    process.env[bootstrapEnv] = '1';

    expect(resolveLocalVitePlusBin(projectDir, globalBinPath)).toBeNull();
    expect(getDelegatedBinPath(projectDir, globalBinPath)).toBe(globalBinPath);
  });
});
