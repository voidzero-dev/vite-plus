import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { detectConfigs } from '../detector.js';

describe('detectConfigs', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.realpathSync(mkdtempSync(path.join(os.tmpdir(), 'vite-config-detect-')));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it('detects nuxt config files as supported config entries', () => {
    fs.writeFileSync(path.join(tempDir, 'nuxt.config.ts'), '');

    expect(detectConfigs(tempDir).viteConfig).toBe('nuxt.config.ts');
  });

  it('prefers vite config files over nuxt config files', () => {
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    fs.writeFileSync(path.join(tempDir, 'nuxt.config.ts'), '');

    expect(detectConfigs(tempDir).viteConfig).toBe('vite.config.ts');
  });
});
