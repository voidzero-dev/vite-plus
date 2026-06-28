import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMigrationReport } from '../report.js';

// Hoisted mock fns so the vi.mock factories (hoisted above imports) can close
// over them and tests can program/inspect them per case.
const { resolveProjectNodeVersion, resolveSupportedNodeVersion, confirm } = vi.hoisted(() => ({
  // resolveProjectNodeVersion → the effective pin { version, source, sourcePath }.
  resolveProjectNodeVersion: vi.fn(),
  // resolveSupportedNodeVersion → the upgrade target, or null when in-range.
  resolveSupportedNodeVersion: vi.fn(),
  // prompts.confirm → the interactive Yes/No answer.
  confirm: vi.fn(),
}));

// Partially mock the native binding: keep every real export, but stub the two
// Node-version resolvers so reading/range-intersection is fully driven by tests.
vi.mock('../../../binding/index.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../../binding/index.js')>();
  return { ...mod, resolveProjectNodeVersion, resolveSupportedNodeVersion };
});

// Partially mock the prompts module: keep everything real (incl. the real
// isCancel, which returns false for plain booleans) but stub confirm.
vi.mock('@voidzero-dev/vite-plus-prompts', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@voidzero-dev/vite-plus-prompts')>();
  return { ...mod, confirm };
});

const { upgradeUnsupportedNodeVersions } = await import('../migrator/setup.js');

const tempDirs: string[] = [];
function makeTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-node-upgrade-'));
  tempDirs.push(dir);
  return dir;
}

function readPkg(dir: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
}

beforeEach(() => {
  resolveProjectNodeVersion.mockReset();
  resolveSupportedNodeVersion.mockReset();
  confirm.mockReset();
  confirm.mockResolvedValue(true);
});

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

describe('upgradeUnsupportedNodeVersions', () => {
  it('upgrades a below-range .node-version (24.2 → 24.18.0) without prompting in non-interactive mode', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.2\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.2',
      source: 'node-version-file',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    const report = createMigrationReport();

    const changed = await upgradeUnsupportedNodeVersions(dir, false, report);

    expect(changed).toBe(true);
    expect(confirm).not.toHaveBeenCalled();
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.18.0\n');
    expect(report.warnings).toContain(
      'Upgraded Node.js 24.2 to 24.18.0 (below the supported range)',
    );
  });

  it('upgrades when interactive and the user confirms', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.3.0\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'node-version-file',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    confirm.mockResolvedValue(true);

    const changed = await upgradeUnsupportedNodeVersions(dir, true);

    expect(confirm).toHaveBeenCalledTimes(1);
    expect(changed).toBe(true);
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.18.0\n');
  });

  it('leaves the pin unchanged when interactive and the user declines', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.3.0\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'node-version-file',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    confirm.mockResolvedValue(false);
    const report = createMigrationReport();

    const changed = await upgradeUnsupportedNodeVersions(dir, true, report);

    expect(confirm).toHaveBeenCalledTimes(1);
    expect(changed).toBe(false);
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.3.0\n');
    expect(report.warnings).toHaveLength(0);
  });

  it('writes nothing when the resolved pin is already supported', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.18.0\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.18.0',
      source: 'node-version-file',
      sourcePath,
    });
    // In-range → the binding reports no upgrade.
    resolveSupportedNodeVersion.mockResolvedValue(null);
    const report = createMigrationReport();

    const changed = await upgradeUnsupportedNodeVersions(dir, false, report);

    expect(changed).toBe(false);
    expect(confirm).not.toHaveBeenCalled();
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.18.0\n');
    expect(report.warnings).toHaveLength(0);
  });

  it('writes nothing when no version source is found', async () => {
    const dir = makeTempDir();
    resolveProjectNodeVersion.mockResolvedValue(null);

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(false);
    expect(resolveSupportedNodeVersion).not.toHaveBeenCalled();
  });

  it('pauses the migration progress spinner before the confirm prompt renders', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.3.0\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'node-version-file',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');

    // Record call order: the spinner must be cleared BEFORE confirm renders,
    // otherwise it animates underneath the prompt.
    const order: string[] = [];
    const pauseProgress = vi.fn(() => order.push('pause'));
    confirm.mockImplementation(async () => {
      order.push('confirm');
      return true;
    });

    await upgradeUnsupportedNodeVersions(dir, true, undefined, pauseProgress);

    expect(pauseProgress).toHaveBeenCalledTimes(1);
    expect(order).toEqual(['pause', 'confirm']);
  });

  it('does not pause the progress spinner when non-interactive (no prompt)', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.3.0\n');
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'node-version-file',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    const pauseProgress = vi.fn();

    await upgradeUnsupportedNodeVersions(dir, false, undefined, pauseProgress);

    expect(pauseProgress).not.toHaveBeenCalled();
    expect(confirm).not.toHaveBeenCalled();
  });

  it('upgrades an engines.node pin in package.json', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, 'package.json');
    fs.writeFileSync(
      sourcePath,
      `${JSON.stringify({ name: 'x', engines: { node: '24.3.0' } }, null, 2)}\n`,
    );
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'engines-node',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect((readPkg(dir).engines as { node: string }).node).toBe('24.18.0');
  });

  it('upgrades a devEngines.runtime node entry in package.json', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, 'package.json');
    fs.writeFileSync(
      sourcePath,
      `${JSON.stringify(
        { name: 'x', devEngines: { runtime: [{ name: 'node', version: '24.3.0' }] } },
        null,
        2,
      )}\n`,
    );
    resolveProjectNodeVersion.mockResolvedValue({
      version: '24.3.0',
      source: 'dev-engines-runtime',
      sourcePath,
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect(
      (readPkg(dir).devEngines as { runtime: Array<{ version: string }> }).runtime[0].version,
    ).toBe('24.18.0');
  });
});
