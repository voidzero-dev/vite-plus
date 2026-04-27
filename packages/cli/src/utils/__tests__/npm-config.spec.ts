import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getNpmAuthHeader, getNpmRegistry } from '../npm-config.js';

describe('getNpmRegistry / getNpmAuthHeader', () => {
  let homeDir: string;
  let projectDir: string;
  let originalEnv: Record<string, string | undefined>;

  beforeEach(() => {
    homeDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-npm-home-'));
    projectDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-npm-proj-'));
    originalEnv = {};
    for (const key of Object.keys(process.env)) {
      if (key.toLowerCase().startsWith('npm_config_')) {
        originalEnv[key] = process.env[key];
        delete process.env[key];
      }
    }
    originalEnv.HOME = process.env.HOME;
    process.env.HOME = homeDir;
    vi.spyOn(os, 'homedir').mockReturnValue(homeDir);
    vi.spyOn(process, 'cwd').mockReturnValue(projectDir);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    fs.rmSync(homeDir, { recursive: true, force: true });
    fs.rmSync(projectDir, { recursive: true, force: true });
    for (const [k, v] of Object.entries(originalEnv)) {
      if (v === undefined) {
        delete process.env[k];
      } else {
        process.env[k] = v;
      }
    }
  });

  it('falls back to the public registry when nothing is configured', () => {
    expect(getNpmRegistry()).toBe('https://registry.npmjs.org');
  });

  it('reads project-level `.npmrc` when it exists', () => {
    fs.writeFileSync(path.join(projectDir, '.npmrc'), 'registry=https://proj.example.com/\n');
    expect(getNpmRegistry()).toBe('https://proj.example.com');
  });

  it('gives project `.npmrc` precedence over user `.npmrc`', () => {
    fs.writeFileSync(path.join(homeDir, '.npmrc'), 'registry=https://user.example.com/\n');
    fs.writeFileSync(path.join(projectDir, '.npmrc'), 'registry=https://proj.example.com/\n');
    expect(getNpmRegistry()).toBe('https://proj.example.com');
  });

  it('uses the user `.npmrc` when the project has none', () => {
    fs.writeFileSync(path.join(homeDir, '.npmrc'), 'registry=https://user.example.com/\n');
    expect(getNpmRegistry()).toBe('https://user.example.com');
  });

  it('resolves `@scope:registry=` overrides ahead of the default', () => {
    fs.writeFileSync(
      path.join(projectDir, '.npmrc'),
      [
        'registry=https://default.example.com/',
        '@your-org:registry=https://scoped.example.com/',
      ].join('\n') + '\n',
    );
    expect(getNpmRegistry('@your-org')).toBe('https://scoped.example.com');
    expect(getNpmRegistry('@other')).toBe('https://default.example.com');
  });

  it('extracts `_authToken` credentials for a matching host', () => {
    fs.writeFileSync(path.join(projectDir, '.npmrc'), '//private.example.com/:_authToken=SECRET\n');
    expect(getNpmAuthHeader('https://private.example.com/some/path')).toBe('Bearer SECRET');
  });

  it('returns undefined when no credential matches the URL host', () => {
    fs.writeFileSync(path.join(projectDir, '.npmrc'), '//private.example.com/:_authToken=SECRET\n');
    expect(getNpmAuthHeader('https://other.example.com/pkg')).toBeUndefined();
  });
});
