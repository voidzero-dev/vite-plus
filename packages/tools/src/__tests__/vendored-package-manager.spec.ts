import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { expect, test } from 'vitest';

import { alignVendoredPackageManagers } from '../vendored-package-manager.ts';

function setup(root: string, files: Record<string, unknown>) {
  for (const [file, contents] of Object.entries(files)) {
    const path = join(root, file);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, JSON.stringify(contents, null, 2) + '\n');
  }
}

function readPackageManager(root: string, file: string) {
  return JSON.parse(readFileSync(join(root, file), 'utf8')).packageManager;
}

test('aligns differing vendored packageManager pins with the root pin', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-vendored-pm-'));
  try {
    setup(root, {
      'package.json': { name: 'root', packageManager: 'pnpm@11.24.0' },
      'rolldown/package.json': { name: 'rolldown', packageManager: 'pnpm@12.4.2' },
      'vite/package.json': { name: 'vite', packageManager: 'pnpm@12.9.1' },
    });
    alignVendoredPackageManagers(root);
    expect(readPackageManager(root, 'rolldown/package.json')).toBe('pnpm@11.24.0');
    expect(readPackageManager(root, 'vite/package.json')).toBe('pnpm@11.24.0');

    // Repeated runs are a no-op.
    alignVendoredPackageManagers(root);
    expect(readPackageManager(root, 'rolldown/package.json')).toBe('pnpm@11.24.0');
    expect(readPackageManager(root, 'vite/package.json')).toBe('pnpm@11.24.0');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('skips missing vendored repos and manifests without a packageManager field', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-vendored-pm-'));
  try {
    setup(root, {
      'package.json': { name: 'root', packageManager: 'pnpm@11.24.0' },
      // rolldown/ is not checked out at all; vite never declared a pin.
      'vite/package.json': { name: 'vite' },
    });
    expect(() => alignVendoredPackageManagers(root)).not.toThrow();
    expect(readPackageManager(root, 'vite/package.json')).toBeUndefined();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('requires the root manifest to declare a packageManager pin', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-vendored-pm-'));
  try {
    setup(root, { 'package.json': { name: 'root' } });
    expect(() => alignVendoredPackageManagers(root)).toThrow(/packageManager/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
