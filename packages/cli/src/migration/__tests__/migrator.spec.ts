import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.js';

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
});
