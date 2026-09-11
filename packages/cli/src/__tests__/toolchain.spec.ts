import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from 'vitest';

import type { ToolchainManifest } from '../../dist/toolchain.js';

const cliPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../..');
const distDir = path.join(cliPkgDir, 'dist');

describe('toolchain export', () => {
  it('generates JSON, JavaScript, and type declaration artifacts', () => {
    for (const file of ['toolchain.json', 'toolchain.js', 'toolchain.d.ts']) {
      expect(fs.existsSync(path.join(distDir, file)), `${file} should exist`).toBe(true);
    }
  });

  it('exports the same manifest as the JSON artifact', async () => {
    const json = JSON.parse(
      fs.readFileSync(path.join(distDir, 'toolchain.json'), 'utf8'),
    ) as ToolchainManifest;
    const module = await import('../../dist/toolchain.js');

    expect(module.toolchain).toEqual(json);
    expect(module.default).toBe(module.toolchain);
  });

  it('contains every required toolchain component with exact versions', async () => {
    const { toolchain } = await import('../../dist/toolchain.js');
    const nodes = new Map(toolchain.nodes.map((node) => [node.id, node]));

    expect([...nodes.keys()]).toEqual([
      'vite-plus',
      'vite-plus-core',
      'vite',
      'rolldown',
      'vitest',
      'oxlint',
      'oxfmt',
      'oxlint-tsgolint',
      'tsdown',
      'vite-task',
      'oxc',
      'oxc-resolver',
    ]);
    for (const node of nodes.values()) {
      if (node.id === 'vite-task') {
        continue;
      }
      expect(node.version, `${node.id} should have an exact version`).toMatch(/^\d+\.\d+\.\d+/);
    }
    expect(nodes.get('vite-task')?.version).toBeUndefined();
    expect(nodes.get('vite-task')?.revision).toMatch(/^[0-9a-f]{40}$/);
    const nativeBuildTimePath = path.join(cliPkgDir, 'binding', 'vite-plus.build-time');
    const sourceDateEpoch = process.env.SOURCE_DATE_EPOCH;
    const expectedBuildTime =
      sourceDateEpoch !== undefined
        ? new Date(Number(sourceDateEpoch) * 1_000).toISOString().replace(/\.\d{3}Z$/, 'Z')
        : fs.existsSync(nativeBuildTimePath)
          ? fs.readFileSync(nativeBuildTimePath, 'utf8').trim()
          : undefined;
    expect(nodes.get('vite-task')?.builtAt).toBe(expectedBuildTime);
  });

  it('uses unambiguous filter labels', async () => {
    const { toolchain } = await import('../../dist/toolchain.js');
    const labels = new Map<string, string>();

    for (const node of toolchain.nodes) {
      for (const label of [node.id, node.name, ...node.aliases]) {
        const existingNode = labels.get(label);
        expect(
          existingNode === undefined || existingNode === node.id,
          `${label} should resolve to one node`,
        ).toBe(true);
        labels.set(label, node.id);
      }
    }
  });

  it('derives the versions export from manifest nodes', async () => {
    const [{ toolchain }, { versions }] = await Promise.all([
      import('../../dist/toolchain.js'),
      import('../../dist/versions.js'),
    ]);
    const nodes = new Map(toolchain.nodes.map((node) => [node.name, node.version]));

    for (const [name, version] of Object.entries(versions)) {
      expect(version, `versions.${name} should match the manifest`).toBe(nodes.get(name));
    }
  });

  it('declares the public manifest schema', () => {
    const declarations = fs.readFileSync(path.join(distDir, 'toolchain.d.ts'), 'utf8');
    expect(declarations).toContain('export interface ToolchainManifest');
    expect(declarations).toContain('readonly schemaVersion: 1');
    expect(declarations).toContain('readonly builtAt?: string');
    expect(declarations).toContain('export default toolchain');
  });
});
