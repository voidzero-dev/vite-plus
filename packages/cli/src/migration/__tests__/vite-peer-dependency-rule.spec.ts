import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parse as parseYaml } from 'yaml';

import { PackageManager, type WorkspaceInfo } from '../../types/index.js';
import { VITE_PLUS_OVERRIDE_PACKAGES } from '../../utils/constants.js';
import type { PnpmPeerDependencyRules as PeerDependencyRules } from '../migrator/shared.js';
import { createMigrationReport } from '../report.js';

// Keep the vite-plus spec stable; each test selects the core alias it needs.
vi.mock('../../utils/constants.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/constants.js')>();
  return {
    ...mod,
    VITE_PLUS_VERSION: 'latest',
    VITE_PLUS_OVERRIDE_PACKAGES: { ...mod.VITE_PLUS_OVERRIDE_PACKAGES },
  };
});

const {
  detectVitePlusBootstrapPending,
  ensureVitePlusBootstrap,
  rewriteStandaloneProject,
  vitePeerDependencyRuleRequired,
} = await import('../migrator.js');

const STABLE_CORE = 'npm:@voidzero-dev/vite-plus-core@8.3.5000';
const PREVIEW_CORE = 'npm:@voidzero-dev/vite-plus-core@0.0.0-commit.0c515e3';

function makeWorkspaceInfo(rootDir: string, version = '10.33.0'): WorkspaceInfo {
  return {
    rootDir,
    isMonorepo: false,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packageManager: PackageManager.pnpm,
    packageManagerVersion: version,
    downloadPackageManager: {
      name: PackageManager.pnpm,
      installDir: '/tmp',
      binPrefix: '/tmp/bin',
      packageName: PackageManager.pnpm,
      version,
    },
    packages: [],
  };
}

function readJson(filePath: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readWorkspaceRules(projectPath: string): PeerDependencyRules | undefined {
  const workspace = parseYaml(
    fs.readFileSync(path.join(projectPath, 'pnpm-workspace.yaml'), 'utf8'),
  ) as { peerDependencyRules?: PeerDependencyRules };
  return workspace.peerDependencyRules;
}

function readPackageJsonRules(projectPath: string): PeerDependencyRules | undefined {
  const pkg = readJson(path.join(projectPath, 'package.json')) as {
    pnpm?: { peerDependencyRules?: PeerDependencyRules };
  };
  return pkg.pnpm?.peerDependencyRules;
}

function writeVitePlusProject(projectPath: string, peerDependencyRules: string[]) {
  fs.writeFileSync(
    path.join(projectPath, 'package.json'),
    JSON.stringify({
      name: 'test',
      devDependencies: { 'vite-plus': 'catalog:' },
      devEngines: {
        packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
      },
    }),
  );
  fs.writeFileSync(
    path.join(projectPath, 'pnpm-workspace.yaml'),
    [
      'catalog:',
      `  vite: ${VITE_PLUS_OVERRIDE_PACKAGES.vite}`,
      '  vite-plus: latest',
      'overrides:',
      "  vite@*: 'catalog:'",
      ...peerDependencyRules,
      '',
    ].join('\n'),
  );
}

describe('vitePeerDependencyRuleRequired', () => {
  it.each([
    [STABLE_CORE, false],
    ['npm:@voidzero-dev/vite-plus-core@9.0.0', false],
    ['npm:@voidzero-dev/vite-plus-core@8.4.0-vp.0', true],
    ['npm:@voidzero-dev/vite-plus-core@9.0.0-beta.3.vp.0', true],
    ['npm:@voidzero-dev/vite-plus-core@0.0.0', true],
    [PREVIEW_CORE, true],
    ['npm:@voidzero-dev/vite-plus-core@1.4.0', true],
    ['npm:@voidzero-dev/vite-plus-core@8.3.5000+build.1', true],
    ['npm:@voidzero-dev/vite-plus-core@latest', true],
    ['file:../voidzero-dev-vite-plus-core-8.3.5999.tgz', true],
    ['https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@1891', true],
  ])('%s requires the rule: %s', (spec, required) => {
    expect(vitePeerDependencyRuleRequired(spec, undefined)).toBe(required);
  });
});

describe('vitePeerDependencyRuleRequired with a pnpm version', () => {
  // pnpm before 10.2.0 applies the `vite@*` alias override to peer ranges, so
  // every `vite` peer fails without the rule, whatever the core version.
  it.each([
    ['9.15.9', true],
    ['10.0.0', true],
    ['10.1.0', true],
    ['10.2.0', false],
    ['10.33.0', false],
    ['12.11.2', false],
    [undefined, false],
  ])('pnpm %s requires the rule for a stable core: %s', (pnpmVersion, required) => {
    expect(vitePeerDependencyRuleRequired(STABLE_CORE, pnpmVersion)).toBe(required);
  });
});

describe('pnpm vite peer rule', () => {
  let tmpDir: string;
  const savedViteOverride = VITE_PLUS_OVERRIDE_PACKAGES.vite;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-vite-peer-rule-'));
  });

  afterEach(() => {
    VITE_PLUS_OVERRIDE_PACKAGES.vite = savedViteOverride;
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  describe('with a stable core version', () => {
    beforeEach(() => {
      VITE_PLUS_OVERRIDE_PACKAGES.vite = STABLE_CORE;
    });

    it('removes the vite rule from pnpm-workspace.yaml and keeps other entries', () => {
      writeVitePlusProject(tmpDir, [
        'peerDependencyRules:',
        '  allowAny:',
        '    - react',
        '    - vite',
        '  allowedVersions:',
        "    react: '*'",
        "    vite: '*'",
      ]);
      const report = createMigrationReport();

      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir), report);

      expect(readWorkspaceRules(tmpDir)).toEqual({
        allowAny: ['react'],
        allowedVersions: { react: '*' },
      });
      expect(report.vitePeerDependencyRuleRemoved).toBe(true);
      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
      expect(ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir)).changed).toBe(false);
    });

    it('deletes peerDependencyRules from pnpm-workspace.yaml when no rule remains', () => {
      writeVitePlusProject(tmpDir, [
        'peerDependencyRules:',
        '  allowAny: [vite, vitest]',
        '  allowedVersions:',
        "    vite: '*'",
        "    vitest: '*'",
      ]);
      const report = createMigrationReport();

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir), report);

      expect(readWorkspaceRules(tmpDir)).toBeUndefined();
      expect(report.vitePeerDependencyRuleRemoved).toBe(true);
      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
    });

    it('writes no peer rule for a project without one', () => {
      writeVitePlusProject(tmpDir, []);
      const report = createMigrationReport();

      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir), report);

      expect(readWorkspaceRules(tmpDir)).toBeUndefined();
      expect(report.vitePeerDependencyRuleRemoved).toBe(false);
    });

    it('keeps the vite rule on pnpm before 10.2.0', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({
          name: 'test',
          devDependencies: { 'vite-plus': 'latest' },
          pnpm: {
            overrides: { react: '18.3.1' },
            peerDependencyRules: { allowAny: ['vite'], allowedVersions: { vite: '*' } },
          },
        }),
      );
      const report = createMigrationReport();

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, '9.15.9'), report);

      expect(readPackageJsonRules(tmpDir)).toEqual({
        allowAny: ['vite'],
        allowedVersions: { vite: '*' },
      });
      expect(report.vitePeerDependencyRuleRemoved).toBe(false);
    });

    it('keeps a user vite range in pnpm-workspace.yaml', () => {
      writeVitePlusProject(tmpDir, [
        'peerDependencyRules:',
        '  allowAny:',
        '    - vite',
        '  allowedVersions:',
        "    vite: '7 || 8'",
      ]);
      const report = createMigrationReport();

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir), report);

      expect(readWorkspaceRules(tmpDir)).toEqual({ allowedVersions: { vite: '7 || 8' } });
      expect(report.vitePeerDependencyRuleRemoved).toBe(true);
      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
    });

    it('keeps a user vite range in package.json before pnpm 10.6.2', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({
          name: 'test',
          devDependencies: { 'vite-plus': 'latest' },
          pnpm: {
            overrides: { react: '18.3.1' },
            peerDependencyRules: { allowedVersions: { vite: '^8.0.0' } },
          },
        }),
      );
      const report = createMigrationReport();

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, '10.6.1'), report);

      expect(readPackageJsonRules(tmpDir)).toEqual({ allowedVersions: { vite: '^8.0.0' } });
      expect(report.vitePeerDependencyRuleRemoved).toBe(false);
    });

    it('removes the vite rule from package.json before pnpm 10.6.2', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({
          name: 'test',
          devDependencies: { 'vite-plus': 'latest' },
          pnpm: {
            overrides: { react: '18.3.1' },
            peerDependencyRules: {
              allowAny: ['react', 'vite'],
              allowedVersions: { vite: '*' },
            },
          },
        }),
      );
      const report = createMigrationReport();

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, '10.6.1'), report);

      expect(readPackageJsonRules(tmpDir)).toEqual({ allowAny: ['react'] });
      expect(report.vitePeerDependencyRuleRemoved).toBe(true);
    });

    it('deletes package.json peerDependencyRules when no rule remains', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({
          name: 'test',
          devDependencies: { 'vite-plus': 'latest' },
          pnpm: {
            peerDependencyRules: { allowAny: ['vite'], allowedVersions: { vite: '*' } },
          },
        }),
      );

      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, '10.6.1'));

      const pkg = readJson(path.join(tmpDir, 'package.json')) as {
        pnpm: Record<string, unknown>;
      };
      expect(pkg.pnpm.peerDependencyRules).toBeUndefined();
      expect(pkg.pnpm.overrides).toEqual({ 'vite@*': STABLE_CORE });
    });

    it('removes a vite rule during the full migration', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({
          name: 'app',
          devDependencies: { vite: '^7.0.0' },
          pnpm: {
            peerDependencyRules: {
              allowAny: ['react', 'vite'],
              allowedVersions: { react: '*', vite: '*' },
            },
          },
        }),
      );
      const report = createMigrationReport();

      rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, '10.6.1'), true, true, report);

      expect(readPackageJsonRules(tmpDir)).toEqual({
        allowAny: ['react'],
        allowedVersions: { react: '*' },
      });
      expect(report.vitePeerDependencyRuleRemoved).toBe(true);
    });
  });

  describe('with a preview core version', () => {
    beforeEach(() => {
      VITE_PLUS_OVERRIDE_PACKAGES.vite = PREVIEW_CORE;
    });

    it('keeps writing the vite rule', () => {
      writeVitePlusProject(tmpDir, []);
      const report = createMigrationReport();

      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
      ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir), report);

      expect(readWorkspaceRules(tmpDir)).toEqual({
        allowAny: ['vite'],
        allowedVersions: { vite: '*' },
      });
      expect(report.vitePeerDependencyRuleRemoved).toBe(false);
      expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
    });
  });
});
