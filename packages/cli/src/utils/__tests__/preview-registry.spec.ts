import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { isPreviewVitePlusVersion, reconcilePreviewBridgeRegistry } from '../preview-registry.ts';

const PREVIEW = '0.0.0-commit.81dee3abe99a61a28ebf112da4f76f2a32aec8b4';
const BRIDGE = 'https://registry-bridge.viteplus.dev/';

describe('isPreviewVitePlusVersion', () => {
  it('flags 0.0.0-<prerelease> preview builds', () => {
    expect(isPreviewVitePlusVersion('0.0.0-commit.abc1234')).toBe(true);
    expect(isPreviewVitePlusVersion('0.0.0-pr.1891')).toBe(true);
  });

  it('does not flag real releases or plain 0.0.0', () => {
    expect(isPreviewVitePlusVersion('0.2.1')).toBe(false);
    expect(isPreviewVitePlusVersion('0.0.1')).toBe(false);
    expect(isPreviewVitePlusVersion('0.0.0')).toBe(false);
    expect(isPreviewVitePlusVersion('1.0.0-beta.1')).toBe(false);
  });
});

describe('reconcilePreviewBridgeRegistry', () => {
  let dir: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-preview-registry-'));
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ name: 'demo' }));
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
    delete process.env.VP_REGISTRY_BRIDGE;
  });

  it('writes nothing for a real release', () => {
    expect(reconcilePreviewBridgeRegistry(dir, '0.2.1')).toBe(false);
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(false);
    expect(fs.existsSync(path.join(dir, '.yarnrc.yml'))).toBe(false);
  });

  it('writes the bridge registry to .npmrc for a preview build', () => {
    expect(reconcilePreviewBridgeRegistry(dir, PREVIEW)).toBe(true);
    const npmrc = fs.readFileSync(path.join(dir, '.npmrc'), 'utf8');
    expect(npmrc).toContain(`registry=${BRIDGE}`);
    expect(fs.existsSync(path.join(dir, '.yarnrc.yml'))).toBe(false);
  });

  // PR #1891 review: a non-Yarn package manager must get `.npmrc` even when a
  // stray `.yarnrc.yml` is present, or its install reads `.npmrc` and can't
  // resolve `0.0.0-commit.<sha>` from the default registry.
  it('writes .npmrc for a non-Yarn package manager despite a stray .yarnrc.yml', () => {
    fs.writeFileSync(path.join(dir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    expect(reconcilePreviewBridgeRegistry(dir, PREVIEW, PackageManager.pnpm)).toBe(true);
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(true);
    expect(fs.readFileSync(path.join(dir, '.npmrc'), 'utf8')).toContain(`registry=${BRIDGE}`);
  });

  it('appends to an existing .npmrc without clobbering it', () => {
    fs.writeFileSync(path.join(dir, '.npmrc'), '@scope:registry=https://npm.example.com/\n');
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    const npmrc = fs.readFileSync(path.join(dir, '.npmrc'), 'utf8');
    expect(npmrc).toContain('@scope:registry=https://npm.example.com/');
    expect(npmrc).toContain(`registry=${BRIDGE}`);
  });

  it('is idempotent (no duplicate registry line on a second call)', () => {
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    const npmrc = fs.readFileSync(path.join(dir, '.npmrc'), 'utf8');
    expect(
      npmrc.match(new RegExp(`registry=${BRIDGE.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&')}`, 'g')),
    ).toHaveLength(1);
  });

  it('honors VP_REGISTRY_BRIDGE override', () => {
    process.env.VP_REGISTRY_BRIDGE = 'https://bridge.example.test/';
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    const npmrc = fs.readFileSync(path.join(dir, '.npmrc'), 'utf8');
    expect(npmrc).toContain('registry=https://bridge.example.test/');
  });

  it('writes .yarnrc.yml (not .npmrc) for a Yarn Berry project with .yarnrc.yml', () => {
    fs.writeFileSync(path.join(dir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    const yarnrc = fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8');
    expect(yarnrc).toContain('nodeLinker: node-modules');
    expect(yarnrc).toContain(`npmRegistryServer: ${BRIDGE}`);
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(false);
  });

  it('detects Yarn Berry from a __metadata lockfile', () => {
    fs.writeFileSync(path.join(dir, 'yarn.lock'), '__metadata:\n  version: 8\n');
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    expect(fs.existsSync(path.join(dir, '.yarnrc.yml'))).toBe(true);
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(false);
  });

  it('detects Yarn Berry from a packageManager pin', () => {
    fs.writeFileSync(
      path.join(dir, 'package.json'),
      JSON.stringify({ name: 'demo', packageManager: 'yarn@4.5.0' }),
    );
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    expect(fs.existsSync(path.join(dir, '.yarnrc.yml'))).toBe(true);
  });

  it('uses .npmrc for Yarn Classic (packageManager yarn@1)', () => {
    fs.writeFileSync(
      path.join(dir, 'package.json'),
      JSON.stringify({ name: 'demo', packageManager: 'yarn@1.22.22' }),
    );
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(true);
    expect(fs.existsSync(path.join(dir, '.yarnrc.yml'))).toBe(false);
  });

  it('round-trip: a real release removes the bridge .npmrc a preview run wrote', () => {
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    expect(fs.readFileSync(path.join(dir, '.npmrc'), 'utf8')).toContain('registry-bridge');

    expect(reconcilePreviewBridgeRegistry(dir, '0.2.1')).toBe(true);
    // The file had only our lines, so it is removed entirely.
    expect(fs.existsSync(path.join(dir, '.npmrc'))).toBe(false);
  });

  it('a real release strips only our lines and keeps the user .npmrc content', () => {
    fs.writeFileSync(path.join(dir, '.npmrc'), 'save-exact=true\n');
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    reconcilePreviewBridgeRegistry(dir, '0.2.1');
    const npmrc = fs.readFileSync(path.join(dir, '.npmrc'), 'utf8');
    expect(npmrc).toContain('save-exact=true');
    expect(npmrc).not.toContain('registry-bridge');
  });

  it('a real release leaves a user-owned .npmrc (no marker) untouched', () => {
    fs.writeFileSync(path.join(dir, '.npmrc'), 'registry=https://npm.example.com/\n');
    expect(reconcilePreviewBridgeRegistry(dir, '0.2.1')).toBe(false);
    expect(fs.readFileSync(path.join(dir, '.npmrc'), 'utf8')).toBe(
      'registry=https://npm.example.com/\n',
    );
  });

  it('preserves and restores a custom npmRegistryServer across a preview round-trip', () => {
    fs.writeFileSync(
      path.join(dir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnpmRegistryServer: https://npm.corp.example/\n',
    );

    // Preview run: the bridge replaces the corporate registry...
    expect(reconcilePreviewBridgeRegistry(dir, PREVIEW, PackageManager.yarn)).toBe(true);
    const bridged = fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8');
    expect(bridged).toContain(`npmRegistryServer: ${BRIDGE}`);
    // ...but the original value is stashed, not lost.
    expect(bridged).toContain('https://npm.corp.example/');

    // A second preview run must not clobber the stashed original.
    expect(reconcilePreviewBridgeRegistry(dir, PREVIEW, PackageManager.yarn)).toBe(true);
    expect(fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8')).toContain(
      'https://npm.corp.example/',
    );

    // Real release: the corporate registry comes back.
    expect(reconcilePreviewBridgeRegistry(dir, '0.2.1', PackageManager.yarn)).toBe(true);
    const restored = fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8');
    expect(restored).toContain('npmRegistryServer: https://npm.corp.example/');
    expect(restored).not.toContain(BRIDGE);
    expect(restored).toContain('nodeLinker: node-modules');
  });

  it('a real release removes the bridge npmRegistryServer but keeps other Berry keys', () => {
    fs.writeFileSync(path.join(dir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    reconcilePreviewBridgeRegistry(dir, PREVIEW);
    expect(fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8')).toContain('npmRegistryServer');

    reconcilePreviewBridgeRegistry(dir, '0.2.1');
    const yarnrc = fs.readFileSync(path.join(dir, '.yarnrc.yml'), 'utf8');
    expect(yarnrc).toContain('nodeLinker: node-modules');
    expect(yarnrc).not.toContain('npmRegistryServer');
  });
});
