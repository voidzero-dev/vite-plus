import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parse as parseYaml } from 'yaml';

import { PackageManager } from '../../types/index.js';
import { createMigrationReport } from '../report.js';

// Mock VITE_PLUS_VERSION to a stable value for snapshot tests.
// When tests run via `vp test`, the env var is injected with the actual version,
// which would cause snapshot mismatches.
vi.mock('../../utils/constants.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/constants.js')>();
  return { ...mod, VITE_PLUS_VERSION: 'latest' };
});

const {
  rewritePackageJson,
  rewriteStandaloneProject,
  rewriteMonorepo,
  parseNvmrcVersion,
  detectNodeVersionManagerFile,
  migrateNodeVersionManagerFile,
  detectFramework,
  hasFrameworkShim,
  addFrameworkShim,
  injectCreateDefaultTemplate,
} = await import('../migrator.js');

describe('rewritePackageJson', () => {
  it('should rewrite package.json scripts and extract staged config', async () => {
    const pkg = {
      scripts: {
        test: 'vitest',
        test_run: 'vitest run && vitest --ui',
        lint: 'oxlint',
        lint_config: 'oxlint --config .oxlint.json',
        lint_type_aware: 'oxlint --type-aware',
        fmt: 'oxfmt',
        fmt_config: 'oxfmt --config .oxfmt.json',
        pack: 'tsdown',
        pack_watch: 'tsdown --watch',
        preview: 'vite preview',
        optimize: 'vite optimize',
        build: 'pnpm install && vite build -r && vite run build --watch && tsdown && tsc || exit 1',
        dev: 'vite',
        dev_cjs: 'VITE_CJS_IGNORE_WARNING=true vite',
        dev_cjs_cross_env: 'cross-env VITE_CJS_IGNORE_WARNING=true vite',
        version: 'vite --version',
        version_short: 'vite -v',
        dev_help: 'vite --help && vite -h',
        dev_port: 'vite --port 3000',
        dev_host: 'vite --host 0.0.0.0',
        dev_open: 'vite --open',
        dev_verbose: 'vite --verbose',
        dev_debug: 'vite --debug',
        dev_trace: 'vite --trace',
        dev_profile: 'vite --profile',
        dev_stats: 'vite --stats',
        dev_analyze: 'vite --analyze',
        ready: 'oxlint --fix --type-aware && vitest run && tsdown && oxfmt --fix',
        ready_env:
          'NODE_ENV=test FOO=bar oxlint --fix --type-aware && NODE_ENV=test FOO=bar vitest run && NODE_ENV=test FOO=bar tsdown && NODE_ENV=test FOO=bar oxfmt --fix',
        ready_new:
          'vite install && vite fmt && vite lint --type-aware && vite test -r && vite build -r',
      },
      'lint-staged': {
        '*.js': ['oxlint --fix --type-aware', 'oxfmt --fix'],
        '*.ts': 'oxfmt --fix',
      },
    };
    const extractedStagedConfig = rewritePackageJson(pkg, PackageManager.npm);
    // lint-staged and vite-staged keys should be removed from pkg
    expect(pkg).toMatchSnapshot();
    // Extracted config should have rewritten commands
    expect(extractedStagedConfig).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on standalone project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on pnpm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on npm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on yarn monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.yarn, true);
    expect(pkg).toMatchSnapshot();
  });

  it('preserves named catalog dependency specs in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: 'catalog:vite7',
        vitest: 'catalog:',
      },
      dependencies: {
        vitest: 'catalog:test',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.devDependencies.vitest).toBe('catalog:');
    expect(pkg.dependencies.vitest).toBe('catalog:test');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('uses default catalog specs for non-catalog dependency specs in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: '^7.0.0',
      },
      dependencies: {
        vitest: '^4.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.yarn, true);

    expect(pkg.devDependencies.vite).toBe('catalog:');
    expect(pkg.dependencies.vitest).toBe('catalog:');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('uses override specs for yarn optional dependencies in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: '^7.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
        vitest: 'catalog:test',
      },
    };

    rewritePackageJson(pkg, PackageManager.yarn, true);

    expect(pkg.devDependencies.vite).toBe('catalog:');
    expect(pkg.optionalDependencies.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.optionalDependencies.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('rewrites peer and optional dependency catalog specs in monorepo projects', async () => {
    const pkg = {
      peerDependencies: {
        vite: 'catalog:vite7',
        tsdown: 'catalog:build',
      },
      optionalDependencies: {
        vitest: 'catalog:test',
        oxlint: 'catalog:build',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vite).toBe('*');
    expect(pkg.peerDependencies).not.toHaveProperty('tsdown');
    expect(pkg.optionalDependencies.vitest).toBe('catalog:test');
    expect(pkg.optionalDependencies).not.toHaveProperty('oxlint');
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');
  });

  it('preserves peer dependency ranges', async () => {
    const pkg = {
      peerDependencies: {
        vite: '^7.0.0',
        vitest: '^4.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
    expect(pkg.optionalDependencies.vite).toBe('catalog:');
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');

    const npmPkg = {
      peerDependencies: {
        vite: '^7.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
      },
    };

    rewritePackageJson(npmPkg, PackageManager.npm);

    expect(npmPkg.peerDependencies.vite).toBe('^7.0.0');
    expect(npmPkg.optionalDependencies.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
  });

  it('adds local vitest when only a peer vitest exists for vitest-adjacent packages', async () => {
    const pkg = {
      dependencies: {
        'vitest-browser-svelte': '^1.0.0',
      },
      peerDependencies: {
        vitest: '^4.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
    expect((pkg as { devDependencies?: Record<string, string> }).devDependencies?.vitest).toBe(
      'catalog:',
    );
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');
  });

  it('should preserve playwright when removing @vitest/browser-playwright', async () => {
    const pkg = {
      devDependencies: {
        '@vitest/browser': '^4.0.0',
        '@vitest/browser-playwright': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.devDependencies).toHaveProperty('playwright', '*');
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser');
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser-playwright');
  });

  it('should preserve webdriverio when removing @vitest/browser-webdriverio', async () => {
    const pkg = {
      devDependencies: {
        '@vitest/browser': '^4.0.0',
        '@vitest/browser-webdriverio': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.devDependencies).toHaveProperty('webdriverio', '*');
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser-webdriverio');
  });

  it('should not overwrite playwright if already in devDependencies', async () => {
    const pkg = {
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        playwright: '^1.40.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.devDependencies).toHaveProperty('playwright', '^1.40.0');
  });

  it('should not add playwright if already in dependencies', async () => {
    const pkg = {
      dependencies: {
        playwright: '^1.40.0',
      },
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.dependencies).toHaveProperty('playwright', '^1.40.0');
    expect(pkg.devDependencies).not.toHaveProperty('playwright');
  });
});

describe('parseNvmrcVersion', () => {
  it('strips v prefix', () => {
    expect(parseNvmrcVersion('v20.5.0')).toBe('20.5.0');
  });

  it('passes through version without prefix', () => {
    expect(parseNvmrcVersion('20.5.0')).toBe('20.5.0');
    expect(parseNvmrcVersion('20')).toBe('20');
  });

  it('passes through lts aliases', () => {
    expect(parseNvmrcVersion('lts/*')).toBe('lts/*');
    expect(parseNvmrcVersion('lts/iron')).toBe('lts/iron');
    expect(parseNvmrcVersion('lts/-1')).toBe('lts/-1');
  });

  it('converts node/stable aliases to lts/*', () => {
    expect(parseNvmrcVersion('node')).toBe('lts/*');
    expect(parseNvmrcVersion('stable')).toBe('lts/*');
  });

  it('returns null for untranslatable aliases', () => {
    expect(parseNvmrcVersion('iojs')).toBeNull();
    expect(parseNvmrcVersion('system')).toBeNull();
    expect(parseNvmrcVersion('default')).toBeNull();
    expect(parseNvmrcVersion('')).toBeNull();
  });

  it('returns null for invalid version strings', () => {
    expect(parseNvmrcVersion('v')).toBeNull();
    expect(parseNvmrcVersion('laetst')).toBeNull();
    expect(parseNvmrcVersion('20.5.0.1')).toBeNull();
  });
});

describe('detectNodeVersionManagerFile', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('returns undefined when no version files found', () => {
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });

  it('returns undefined when .node-version already exists', () => {
    fs.writeFileSync(path.join(tmpDir, '.node-version'), '20.5.0\n');
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });

  it('detects .nvmrc', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({ file: '.nvmrc' });
  });

  it('detects volta node in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '20.5.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({
      file: 'package.json',
      voltaNodeVersion: '20.5.0',
    });
  });

  it('prefers .nvmrc over volta when both are present and sets voltaPresent', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '18.0.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({ file: '.nvmrc', voltaPresent: true });
  });

  it('returns undefined when .node-version already exists even with volta', () => {
    fs.writeFileSync(path.join(tmpDir, '.node-version'), '20.5.0\n');
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '20.5.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });
});

describe('migrateNodeVersionManagerFile', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('adds volta manual step when voltaPresent is set', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    const report = {
      createdViteConfigCount: 0,
      mergedConfigCount: 0,
      mergedStagedConfigCount: 0,
      inlinedLintStagedConfigCount: 0,
      removedConfigCount: 0,
      tsdownImportCount: 0,
      rewrittenImportFileCount: 0,
      rewrittenImportErrors: [],
      eslintMigrated: false,
      prettierMigrated: false,
      nodeVersionFileMigrated: false,
      gitHooksConfigured: false,
      frameworkShimAdded: false,
      warnings: [],
      manualSteps: [],
    };
    migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc', voltaPresent: true }, report);
    expect(report.manualSteps).toContain('Remove the "volta" field from package.json');
  });

  it('migrates .nvmrc to .node-version and removes .nvmrc', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    const ok = migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc' });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('20.5.0\n');
    expect(fs.existsSync(path.join(tmpDir, '.nvmrc'))).toBe(false);
  });

  it('returns false and warns for unsupported alias', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'system\n');
    const report = {
      createdViteConfigCount: 0,
      mergedConfigCount: 0,
      mergedStagedConfigCount: 0,
      inlinedLintStagedConfigCount: 0,
      removedConfigCount: 0,
      tsdownImportCount: 0,
      rewrittenImportFileCount: 0,
      rewrittenImportErrors: [],
      eslintMigrated: false,
      prettierMigrated: false,
      nodeVersionFileMigrated: false,
      gitHooksConfigured: false,
      frameworkShimAdded: false,
      warnings: [],
      manualSteps: [],
    };
    const ok = migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc' }, report);
    expect(ok).toBe(false);
    expect(report.warnings.length).toBe(1);
    expect(fs.existsSync(path.join(tmpDir, '.node-version'))).toBe(false);
  });

  it('migrates volta node version to .node-version', () => {
    const ok = migrateNodeVersionManagerFile(tmpDir, {
      file: 'package.json',
      voltaNodeVersion: '20.5.0',
    });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('20.5.0\n');
  });

  it('sets nodeVersionFileMigrated and manualSteps in report for volta migration', () => {
    const report = {
      createdViteConfigCount: 0,
      mergedConfigCount: 0,
      mergedStagedConfigCount: 0,
      inlinedLintStagedConfigCount: 0,
      removedConfigCount: 0,
      tsdownImportCount: 0,
      rewrittenImportFileCount: 0,
      rewrittenImportErrors: [],
      eslintMigrated: false,
      prettierMigrated: false,
      nodeVersionFileMigrated: false,
      gitHooksConfigured: false,
      frameworkShimAdded: false,
      warnings: [],
      manualSteps: [],
    };
    migrateNodeVersionManagerFile(
      tmpDir,
      { file: 'package.json', voltaNodeVersion: '20.5.0' },
      report,
    );
    expect(report.nodeVersionFileMigrated).toBe(true);
    expect(report.manualSteps).toContain('Remove the "volta" field from package.json');
  });

  it('normalizes volta.node "lts" to "lts/*"', () => {
    const ok = migrateNodeVersionManagerFile(tmpDir, {
      file: 'package.json',
      voltaNodeVersion: 'lts',
    });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('lts/*\n');
  });

  it('returns false and warns when volta.node is a partial version', () => {
    const report = {
      createdViteConfigCount: 0,
      mergedConfigCount: 0,
      mergedStagedConfigCount: 0,
      inlinedLintStagedConfigCount: 0,
      removedConfigCount: 0,
      tsdownImportCount: 0,
      rewrittenImportFileCount: 0,
      rewrittenImportErrors: [],
      eslintMigrated: false,
      prettierMigrated: false,
      nodeVersionFileMigrated: false,
      gitHooksConfigured: false,
      frameworkShimAdded: false,
      warnings: [],
      manualSteps: [],
    };
    const ok = migrateNodeVersionManagerFile(
      tmpDir,
      { file: 'package.json', voltaNodeVersion: '20' },
      report,
    );
    expect(ok).toBe(false);
    expect(report.warnings.length).toBe(1);
    expect(fs.existsSync(path.join(tmpDir, '.node-version'))).toBe(false);
  });
});

function makeWorkspaceInfo(
  rootDir: string,
  packageManager: PackageManager,
): import('../../types/index.js').WorkspaceInfo {
  return {
    rootDir,
    isMonorepo: false,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packageManager,
    packageManagerVersion: '10.33.0',
    downloadPackageManager: {
      name: 'pnpm',
      installDir: '/tmp',
      binPrefix: '/tmp/bin',
      packageName: 'pnpm',
      version: '10.33.0',
    },
    packages: [],
  };
}

function readJson(filePath: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readYaml(filePath: string): string {
  return fs.readFileSync(filePath, 'utf8');
}

function readYamlObject(filePath: string): Record<string, unknown> {
  return parseYaml(readYaml(filePath)) as Record<string, unknown>;
}

describe('rewriteStandaloneProject pnpm workspace yaml', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-pnpm-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('creates pnpm-workspace.yaml when no existing pnpm config in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // pnpm-workspace.yaml should be created
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(true);
    const yaml = readYaml(path.join(tmpDir, 'pnpm-workspace.yaml'));
    expect(yaml).toContain('overrides:');
    expect(yaml).toContain('peerDependencyRules:');
    expect(yaml).toContain('catalog:');

    // package.json should not have pnpm section
    const pkg = readJson(path.join(tmpDir, 'package.json'));
    expect(pkg.pnpm).toBeUndefined();

    // devDependencies should use catalog:
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps.vite).toBe('catalog:');
    expect(devDeps['vite-plus']).toBe('catalog:');
  });

  it('keeps pnpm config in package.json when existing pnpm field present', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: {
          overrides: { 'some-pkg': '1.0.0' },
          onlyBuiltDependencies: ['esbuild'],
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // pnpm-workspace.yaml should NOT be created
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);

    // package.json should have pnpm.overrides with both existing and vite overrides
    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const pnpm = pkg.pnpm as Record<string, unknown>;
    expect(pnpm).toBeDefined();
    const overrides = pnpm.overrides as Record<string, string>;
    expect(overrides['some-pkg']).toBe('1.0.0');
    expect(overrides.vite).toBeDefined();
    expect(overrides.vitest).toBeDefined();

    // peerDependencyRules should be present
    expect(pnpm.peerDependencyRules).toBeDefined();
    // onlyBuiltDependencies should be preserved
    expect(pnpm.onlyBuiltDependencies).toEqual(['esbuild']);
  });

  it('preserves custom peerDependencyRules when migrating to pnpm-workspace.yaml', () => {
    // Project has peerDependencyRules but no pnpm.overrides -- pnpm field is present
    // so it should keep using package.json
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: {
          peerDependencyRules: {
            allowAny: ['react', 'vite'],
            allowedVersions: { react: '*', vite: '*' },
            ignoreMissing: ['@types/node'],
          },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const pnpm = pkg.pnpm as Record<string, unknown>;
    const rules = pnpm.peerDependencyRules as Record<string, unknown>;
    // Custom entries preserved, Vite entries merged
    expect(rules.allowAny).toEqual(expect.arrayContaining(['react', 'vite', 'vitest']));
    // ignoreMissing preserved
    expect(rules.ignoreMissing).toEqual(['@types/node']);
  });

  it('writes vite overrides with catalog references to pnpm-workspace.yaml', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYaml(path.join(tmpDir, 'pnpm-workspace.yaml'));
    expect(yaml).toContain("vite: 'catalog:'");
    expect(yaml).toContain("vitest: 'catalog:'");
  });

  it('rewrites named catalogs in pnpm-workspace.yaml without adding new entries', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: 'catalog:vite7' },
        peerDependencies: {
          vite: 'catalog:vite7',
          vitest: 'catalog:',
          tsdown: 'catalog:test',
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'overrides:',
        '  vite: catalog:vite7',
        'catalog:',
        '  vitest: ^4.0.0',
        'catalogs:',
        '  vite7:',
        '    react: ^18.0.0',
        '    vite: ^7.0.0',
        '    vite-plus: ^0.0.0',
        '  test:',
        '    vitest: ^4.0.0',
        '    tsdown: ^0.1.0',
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
      overrides: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yaml.overrides.vite).toBe('catalog:vite7');
    expect(yaml.overrides.vitest).toBe('catalog:');
    expect(yaml.catalog.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(yaml.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(yaml.catalogs.vite7.react).toBe('^18.0.0');
    expect(yaml.catalogs.vite7['vite-plus']).toBe('latest');
    expect(yaml.catalogs.test.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(yaml.catalogs.test.tsdown).toBeUndefined();
    expect(yaml.catalogs.test['vite-plus']).toBeUndefined();

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
    expect(pkg.peerDependencies).not.toHaveProperty('tsdown');
  });

  it('preserves named pnpm overrides when moving root overrides to pnpm-workspace.yaml', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'pnpm-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:vite7' },
        pnpm: {
          overrides: {
            vite: 'catalog:vite7',
            react: '^18.0.0',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['packages:', '  - packages/*', 'catalogs:', '  vite7:', '    vite: ^7.0.0', ''].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      overrides: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yaml.overrides.vite).toBe('catalog:vite7');
    expect(yaml.overrides.vitest).toBe('catalog:');
    expect(yaml.overrides.react).toBe('^18.0.0');
    expect(yaml.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      pnpm?: unknown;
    };
    expect(pkg.pnpm).toBeUndefined();
  });

  it('preserves default pnpm catalog overrides over stale workspace named overrides', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'pnpm-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:' },
        pnpm: {
          overrides: {
            vite: 'catalog:',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'overrides:',
        '  vite: catalog:vite7',
        'catalogs:',
        '  vite7:',
        '    vite: ^7.0.0',
        '',
      ].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      overrides: Record<string, string>;
    };
    expect(yaml.overrides.vite).toBe('catalog:');
    expect(yaml.overrides.vitest).toBe('catalog:');
  });

  it('does not resolve peer dependency catalog specs to migrated aliases', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        peerDependencies: {
          vite: 'catalog:vite7',
          vitest: 'catalog:',
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        'catalogs:',
        '  vite7:',
        '    vite: npm:@voidzero-dev/vite-plus-core@latest',
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      peerDependencies: Record<string, string>;
    };
    expect(pkg.peerDependencies.vite).toBe('*');
    expect(pkg.peerDependencies.vitest).toBe('*');
  });
});

describe('rewriteMonorepo yarn catalog', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-yarn-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('rewrites named catalogs in .yarnrc.yml and keeps nodeLinker', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'yarn-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:vite7' },
        peerDependencies: { vite: 'catalog:vite7', vitest: 'catalog:test' },
        packageManager: 'yarn@4.10.0',
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      [
        'catalogs:',
        '  vite7:',
        '    react: ^18.0.0',
        '    vite: ^7.0.0',
        '  test:',
        '    vitest: ^4.0.0',
        '    oxlint: ^1.0.0',
        '',
      ].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.yarn), true);

    const yarnrc = readYamlObject(path.join(tmpDir, '.yarnrc.yml')) as {
      nodeLinker: string;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yarnrc.nodeLinker).toBe('node-modules');
    expect(yarnrc.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(yarnrc.catalogs.vite7.react).toBe('^18.0.0');
    expect(yarnrc.catalogs.test.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(yarnrc.catalogs.test.oxlint).toBeUndefined();

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
  });
});

describe('rewriteMonorepo bun catalog', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-bun-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('writes catalog to top-level when workspaces is an array', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // catalog should be at top level
    const catalog = pkg.catalog as Record<string, string>;
    expect(catalog.vite).toBeDefined();
    expect(catalog['vite-plus']).toBe('latest');
    // overrides should reference catalog:
    const overrides = pkg.overrides as Record<string, string>;
    expect(overrides.vite).toBe('catalog:');
  });

  it('writes catalog to workspaces.catalog when workspaces is an object with existing catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalog: { react: '^19.0.0' },
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // No top-level catalog
    expect(pkg.catalog).toBeUndefined();
    // workspaces.catalog should have merged entries
    const workspaces = pkg.workspaces as { packages: string[]; catalog: Record<string, string> };
    expect(workspaces.catalog.react).toBe('^19.0.0');
    expect(workspaces.catalog.vite).toBeDefined();
    expect(workspaces.catalog['vite-plus']).toBe('latest');
    // workspaces.packages should be preserved
    expect(workspaces.packages).toEqual(['packages/*']);
  });

  it('cleans stale top-level bun catalog when workspaces.catalog is preferred', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalog: { vite: '^7.0.0' },
        },
        catalog: {
          vite: '^6.0.0',
          vitest: '^3.0.0',
          tsdown: '^0.1.0',
          react: '^19.0.0',
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      workspaces: { catalog: Record<string, string> };
    };
    expect(pkg.workspaces.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog['vite-plus']).toBe('latest');
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.catalog.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(pkg.catalog.tsdown).toBeUndefined();
    expect(pkg.catalog.react).toBe('^19.0.0');
    expect(pkg.catalog['vite-plus']).toBeUndefined();
  });

  it('writes catalog to top-level when workspaces is an object without catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // catalog should be at top level since workspaces.catalog didn't exist
    const catalog = pkg.catalog as Record<string, string>;
    expect(catalog.vite).toBeDefined();
    expect(catalog['vite-plus']).toBe('latest');
    // workspaces object should be preserved
    const workspaces = pkg.workspaces as { packages: string[] };
    expect(workspaces.packages).toEqual(['packages/*']);
  });

  it('rewrites top-level named catalogs and preserves named overrides', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        catalogs: {
          build: { vite: '^7.0.0', react: '^19.0.0', tsdown: '^0.1.0' },
          test: { vitest: '^4.0.0' },
        },
        overrides: { vite: 'catalog:build' },
        devDependencies: { vite: 'catalog:build' },
        peerDependencies: { vite: 'catalog:build', vitest: 'catalog:test' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
      overrides: Record<string, string>;
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.catalogs.build.react).toBe('^19.0.0');
    expect(pkg.catalogs.build.tsdown).toBeUndefined();
    expect(pkg.catalogs.test.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(pkg.overrides.vite).toBe('catalog:build');
    expect(pkg.overrides.vitest).toBe('catalog:');
    expect(pkg.devDependencies.vite).toBe('catalog:build');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
  });

  it('rewrites workspaces named catalogs and writes default catalog beside them', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalogs: {
            build: { vite: '^7.0.0', oxlint: '^1.0.0' },
            test: { vitest: '^4.0.0', vite: '^7.0.0' },
          },
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog?: Record<string, string>;
      workspaces: {
        catalog: Record<string, string>;
        catalogs: Record<string, Record<string, string>>;
      };
      overrides: Record<string, string>;
    };
    expect(pkg.catalog).toBeUndefined();
    expect(pkg.workspaces.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog['vite-plus']).toBe('latest');
    expect(pkg.workspaces.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalogs.build.oxlint).toBeUndefined();
    expect(pkg.workspaces.catalogs.test.vitest).toBe('npm:@voidzero-dev/vite-plus-test@latest');
    expect(pkg.workspaces.catalogs.test.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.overrides.vite).toBe('catalog:');
  });

  it('keeps an existing top-level default catalog when workspaces named catalogs exist', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalogs: {
            build: { vite: '^7.0.0' },
          },
        },
        catalog: { react: '^19.0.0' },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      workspaces: {
        catalog?: Record<string, string>;
        catalogs: Record<string, Record<string, string>>;
      };
    };
    expect(pkg.catalog.react).toBe('^19.0.0');
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog).toBeUndefined();
    expect(pkg.workspaces.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
  });
});

describe('framework shim', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  describe('detectFramework', () => {
    it('returns [vue] when vue is in devDependencies', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['vue']);
    });

    it('returns [astro] when astro is in devDependencies', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { astro: '^4.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['astro']);
    });

    it('returns [vue, astro] when both are present', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0', astro: '^4.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['vue', 'astro']);
    });

    it('returns [] when no framework dependency is present', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vite: '^7.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual([]);
    });

    it('returns [] when package.json does not exist', () => {
      expect(detectFramework(tmpDir)).toEqual([]);
    });
  });

  describe('hasFrameworkShim', () => {
    it('returns true when src/env.d.ts contains vue shim', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      fs.writeFileSync(
        path.join(srcDir, 'env.d.ts'),
        "declare module '*.vue' { export default {} }\n",
      );
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(true);
    });

    it('returns false when src/env.d.ts does not contain vue shim', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      fs.writeFileSync(
        path.join(srcDir, 'env.d.ts'),
        '/// <reference types="vite-plus/client" />\n',
      );
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(false);
    });

    it('returns false when env.d.ts does not exist', () => {
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(false);
    });

    it('returns true when root env.d.ts contains astro/client reference', () => {
      fs.writeFileSync(path.join(tmpDir, 'env.d.ts'), '/// <reference types="astro/client" />\n');
      expect(hasFrameworkShim(tmpDir, 'astro')).toBe(true);
    });
  });

  describe('addFrameworkShim', () => {
    it('creates src/env.d.ts with vue shim when src/ exists and no env.d.ts', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
      expect(content).toContain('DefineComponent');
    });

    it('creates root env.d.ts with vue shim when no src/ dir', () => {
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(tmpDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
    });

    it('appends vue shim to existing src/env.d.ts', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      const existing = '/// <reference types="vite-plus/client" />\n';
      fs.writeFileSync(path.join(srcDir, 'env.d.ts'), existing);
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain('/// <reference types="vite-plus/client" />');
      expect(content).toContain("declare module '*.vue'");
    });

    it('sets frameworkShimAdded on report', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      const report = createMigrationReport();
      addFrameworkShim(tmpDir, 'vue', report);
      expect(report.frameworkShimAdded).toBe(true);
    });
  });

  describe('create flow integration', () => {
    it('does not add duplicate shim when template already wrote env.d.ts', () => {
      // Simulate create-vue having already written a shim into src/env.d.ts
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      const existingShim =
        "declare module '*.vue' {\n  import type { DefineComponent } from 'vue';\n  const component: DefineComponent;\n  export default component;\n}\n";
      fs.writeFileSync(path.join(srcDir, 'env.d.ts'), existingShim);
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );

      expect(detectFramework(tmpDir)).toEqual(['vue']);
      // Gate check: shim already present, so addFrameworkShim should NOT be called
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(true);
      // Verify content is unchanged if caller respects the gate
      const contentBefore = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const contentAfter = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      expect(contentAfter).toBe(contentBefore);
    });

    it('adds shim for vue project created without env.d.ts', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
    });

    it('adds astro shim for astro project without env.d.ts', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { astro: '^4.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain('/// <reference types="astro/client" />');
    });

    it('adds both vue and astro shims for Astro+Vue project', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0', astro: '^4.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
      expect(content).toContain('/// <reference types="astro/client" />');
    });
  });

  describe('injectCreateDefaultTemplate', () => {
    let tmpDir: string;

    beforeEach(() => {
      tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrator-create-default-'));
    });

    afterEach(() => {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    });

    function writeViteConfig(body: string): void {
      fs.writeFileSync(
        path.join(tmpDir, 'vite.config.ts'),
        `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig(${body});\n`,
      );
    }

    it('injects `create.defaultTemplate` when scope is set and no `create:` exists', () => {
      writeViteConfig('{ run: { cache: true } }');
      injectCreateDefaultTemplate(tmpDir, '@your-org', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).toContain('create:');
      expect(content).toContain('"defaultTemplate":"@your-org"');
    });

    it('skips injection when scope is empty (no scope to default to)', () => {
      writeViteConfig('{ run: { cache: true } }');
      injectCreateDefaultTemplate(tmpDir, '', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).not.toContain('create:');
      expect(content).not.toContain('defaultTemplate');
    });

    it('preserves an existing `create:` block instead of overwriting it', () => {
      writeViteConfig("{ create: { defaultTemplate: '@other' }, run: { cache: true } }");
      injectCreateDefaultTemplate(tmpDir, '@your-org', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).toContain("'@other'");
      expect(content).not.toContain('@your-org');
    });
  });
});

describe('rewriteStandaloneProject — tsconfig types rewriting', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsconfig-'));
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('rewrites tsdown/client to vite-plus/pack/client in tsconfig.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { types: ['tsdown/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain(
      'vite-plus/pack/client',
    );
    expect((tsconfig.compilerOptions as { types: string[] }).types).not.toContain('tsdown/client');
  });

  it('rewrites vite/client to vite-plus/client in tsconfig.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { types: ['vite/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain('vite-plus/client');
    expect((tsconfig.compilerOptions as { types: string[] }).types).not.toContain('vite/client');
  });

  it('rewrites types in tsconfig.node.json as well', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.node.json'),
      JSON.stringify({ compilerOptions: { types: ['tsdown/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.node.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain(
      'vite-plus/pack/client',
    );
  });
});
