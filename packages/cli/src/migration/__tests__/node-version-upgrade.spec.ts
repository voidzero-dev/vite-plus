import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMigrationReport } from '../report.js';

// Hoisted mock fns so the vi.mock factories (hoisted above imports) can close
// over them and tests can program/inspect them per case.
const { resolveSupportedNodeVersion, resolveSupportedNodeRange, confirm } = vi.hoisted(() => ({
  // resolveSupportedNodeVersion → the concrete latest release for a major (used
  // for the single-version `.node-version` file), or null when in-range.
  resolveSupportedNodeVersion: vi.fn(),
  // resolveSupportedNodeRange → the `>=<supported-minimum>` open-ended range
  // (used for the `engines.node` / `devEngines.runtime` constraint fields), or
  // null when in-range.
  resolveSupportedNodeRange: vi.fn(),
  // prompts.confirm → the interactive Yes/No answer.
  confirm: vi.fn(),
}));

// Partially mock the native binding: keep every real export, but stub the two
// Node-version resolvers so the upgrade decision is fully driven by tests.
vi.mock('../../../binding/index.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../../binding/index.js')>();
  return { ...mod, resolveSupportedNodeVersion, resolveSupportedNodeRange };
});

// Partially mock the prompts module: keep everything real (incl. the real
// isCancel, which returns false for plain booleans) but stub confirm.
vi.mock('@voidzero-dev/vite-plus-prompts', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@voidzero-dev/vite-plus-prompts')>();
  return { ...mod, confirm };
});

const { upgradeUnsupportedNodeVersions, hasUnsupportedNodeVersionPin } =
  await import('../migrator/setup.js');

const tempDirs: string[] = [];
function makeTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-node-upgrade-'));
  tempDirs.push(dir);
  return dir;
}

function writePkg(dir: string, pkg: Record<string, unknown>) {
  fs.writeFileSync(path.join(dir, 'package.json'), `${JSON.stringify(pkg, null, 2)}\n`);
}

function readPkg(dir: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
}

beforeEach(() => {
  resolveSupportedNodeVersion.mockReset();
  resolveSupportedNodeRange.mockReset();
  // Default: nothing to upgrade. Individual tests override per source.
  resolveSupportedNodeVersion.mockResolvedValue(null);
  resolveSupportedNodeRange.mockReturnValue(null);
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
    // In-range → the binding reports no upgrade.
    resolveSupportedNodeVersion.mockResolvedValue(null);
    const report = createMigrationReport();

    const changed = await upgradeUnsupportedNodeVersions(dir, false, report);

    expect(changed).toBe(false);
    expect(confirm).not.toHaveBeenCalled();
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.18.0\n');
    expect(report.warnings).toHaveLength(0);
  });

  it('leaves a non-version .node-version alias (lts/*) untouched', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, 'lts/*\n');
    // Unparsable alias → the binding reports no upgrade.
    resolveSupportedNodeVersion.mockResolvedValue(null);

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(false);
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('lts/*\n');
  });

  it('writes nothing when no version source is found', async () => {
    const dir = makeTempDir();

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(false);
    expect(resolveSupportedNodeVersion).not.toHaveBeenCalled();
    expect(resolveSupportedNodeRange).not.toHaveBeenCalled();
  });

  it('pauses the migration progress spinner before the confirm prompt renders', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.3.0\n');
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
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    const pauseProgress = vi.fn();

    await upgradeUnsupportedNodeVersions(dir, false, undefined, pauseProgress);

    expect(pauseProgress).not.toHaveBeenCalled();
    expect(confirm).not.toHaveBeenCalled();
  });

  it('rewrites a below-floor engines.node to an open-ended supported range (>=24 → >=24.11.0)', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x', engines: { node: '>=24' } });
    // Constraint fields use the range resolver, not the concrete one.
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect((readPkg(dir).engines as { node: string }).node).toBe('>=24.11.0');
  });

  it('rewrites a below-floor devEngines.runtime node entry to an open-ended supported range (24 → >=24.11.0)', async () => {
    const dir = makeTempDir();
    writePkg(dir, {
      name: 'x',
      devEngines: { runtime: [{ name: 'node', version: '24' }] },
    });
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect(
      (readPkg(dir).devEngines as { runtime: Array<{ version: string }> }).runtime[0].version,
    ).toBe('>=24.11.0');
  });

  it('rewrites a below-floor devEngines.runtime node entry in object (non-array) form', async () => {
    const dir = makeTempDir();
    writePkg(dir, {
      name: 'x',
      devEngines: { runtime: { name: 'node', version: '^24' } },
    });
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect((readPkg(dir).devEngines as { runtime: { version: string } }).runtime.version).toBe(
      '>=24.11.0',
    );
  });

  it('leaves an already-supported engines.node range untouched', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x', engines: { node: '>=24.11.0' } });
    // In-range → the range resolver reports no upgrade.
    resolveSupportedNodeRange.mockReturnValue(null);

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(false);
    expect((readPkg(dir).engines as { node: string }).node).toBe('>=24.11.0');
  });

  it('upgrades all three sources independently in a single pass', async () => {
    const dir = makeTempDir();
    const nodeVersionPath = path.join(dir, '.node-version');
    fs.writeFileSync(nodeVersionPath, '24\n');
    writePkg(dir, {
      name: 'x',
      engines: { node: '24.3.0' },
      devEngines: { runtime: [{ name: 'node', version: '>=24' }] },
    });
    // .node-version → concrete latest of the major.
    resolveSupportedNodeVersion.mockImplementation(async (from: string) =>
      from === '24' ? '24.18.0' : null,
    );
    // Constraint fields → open-ended supported range.
    resolveSupportedNodeRange.mockImplementation((from: string) =>
      from === '>=24' || from === '24.3.0' ? '>=24.11.0' : null,
    );

    const changed = await upgradeUnsupportedNodeVersions(dir, false);

    expect(changed).toBe(true);
    expect(fs.readFileSync(nodeVersionPath, 'utf8')).toBe('24.18.0\n');
    const pkg = readPkg(dir);
    expect((pkg.engines as { node: string }).node).toBe('>=24.11.0');
    expect((pkg.devEngines as { runtime: Array<{ version: string }> }).runtime[0].version).toBe(
      '>=24.11.0',
    );
  });

  it('shows a single confirm covering every planned change in interactive mode', async () => {
    const dir = makeTempDir();
    fs.writeFileSync(path.join(dir, '.node-version'), '24\n');
    writePkg(dir, {
      name: 'x',
      engines: { node: '24.3.0' },
      devEngines: { runtime: [{ name: 'node', version: '>=24' }] },
    });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');
    confirm.mockResolvedValue(true);

    const changed = await upgradeUnsupportedNodeVersions(dir, true);

    expect(changed).toBe(true);
    expect(confirm).toHaveBeenCalledTimes(1);
  });

  it('declining the single confirm leaves all sources unchanged', async () => {
    const dir = makeTempDir();
    const nodeVersionPath = path.join(dir, '.node-version');
    fs.writeFileSync(nodeVersionPath, '24\n');
    writePkg(dir, { name: 'x', engines: { node: '24.3.0' } });
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');
    confirm.mockResolvedValue(false);

    const changed = await upgradeUnsupportedNodeVersions(dir, true);

    expect(changed).toBe(false);
    expect(fs.readFileSync(nodeVersionPath, 'utf8')).toBe('24\n');
    expect((readPkg(dir).engines as { node: string }).node).toBe('24.3.0');
  });
});

// Reachability guard: an already-Vite+, otherwise up-to-date project whose ONLY
// pending work is a below-floor Node pin must NOT hit the "already using Vite+"
// early return in bin.ts. `hasExistingVitePlusMigrationCandidates` gates that
// early return; it now reuses this side-effect-free detector (the same
// `planNodeVersionUpgrades` planner the upgrade itself uses) so the otherwise
// up-to-date project is still routed into the upgrade. Detection must NEVER write
// to disk (the actual rewrite is left to `upgradeUnsupportedNodeVersions`).
describe('hasUnsupportedNodeVersionPin', () => {
  it('detects a below-floor engines.node pin without rewriting it', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x', engines: { node: '>=24' } });
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');

    const detected = await hasUnsupportedNodeVersionPin(dir);

    expect(detected).toBe(true);
    // Detection only: the pin must be left untouched on disk.
    expect((readPkg(dir).engines as { node: string }).node).toBe('>=24');
  });

  it('detects a below-floor devEngines.runtime pin without rewriting it', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x', devEngines: { runtime: [{ name: 'node', version: '24' }] } });
    resolveSupportedNodeRange.mockReturnValue('>=24.11.0');

    const detected = await hasUnsupportedNodeVersionPin(dir);

    expect(detected).toBe(true);
    expect(
      (readPkg(dir).devEngines as { runtime: Array<{ version: string }> }).runtime[0].version,
    ).toBe('24');
  });

  it('detects a below-floor .node-version pin without rewriting it', async () => {
    const dir = makeTempDir();
    const sourcePath = path.join(dir, '.node-version');
    fs.writeFileSync(sourcePath, '24.2\n');
    resolveSupportedNodeVersion.mockResolvedValue('24.18.0');

    const detected = await hasUnsupportedNodeVersionPin(dir);

    expect(detected).toBe(true);
    expect(fs.readFileSync(sourcePath, 'utf8')).toBe('24.2\n');
  });

  it('returns false when the only Node pin is already at/above the supported floor', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x', engines: { node: '>=24.11.0' } });
    // In-range → both resolvers report no upgrade (defaults already do this).
    resolveSupportedNodeRange.mockReturnValue(null);

    expect(await hasUnsupportedNodeVersionPin(dir)).toBe(false);
  });

  it('returns false when the project declares no Node pin at all', async () => {
    const dir = makeTempDir();
    writePkg(dir, { name: 'x' });

    expect(await hasUnsupportedNodeVersionPin(dir)).toBe(false);
    expect(resolveSupportedNodeVersion).not.toHaveBeenCalled();
    expect(resolveSupportedNodeRange).not.toHaveBeenCalled();
  });
});
