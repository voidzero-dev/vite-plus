import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { stringify } from 'yaml';

import {
  createCatalogDependencyResolver,
  rewriteMonorepoProject,
  rewriteYarnrcYml,
  yarnrcSatisfiesVitePlus,
} from '../../migration/migrator.ts';
import { PackageManager } from '../../types/index.ts';
import {
  VITEST_VERSION,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
} from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { readYamlFile } from '../../utils/yaml.ts';
import { ensureCreateCatalogEntries } from '../utils.ts';

let root: string;
let project: string;
let workspaceFile: string;

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-create-catalog-'));
  project = path.join(root, 'apps/web');
  fs.mkdirSync(project, { recursive: true });
  workspaceFile = path.join(root, 'pnpm-workspace.yaml');
  fs.writeFileSync(path.join(root, 'package.json'), '{"private":true}');
  fs.writeFileSync(workspaceFile, 'packages:\n  - apps/*\n');
});

afterEach(() => fs.rmSync(root, { recursive: true, force: true }));

function integrate(dependencies: Record<string, string> = { vite: '^8.0.0' }) {
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({ devDependencies: dependencies }),
  );
  rewriteMonorepoProject(
    project,
    PackageManager.pnpm,
    true,
    true,
    undefined,
    createCatalogDependencyResolver(root, PackageManager.pnpm),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.pnpm);
  return readYamlFile(workspaceFile);
}

it.each([PackageManager.pnpm, PackageManager.yarn, PackageManager.bun])(
  'completes aligned Vitest entries in the selected %s catalog without replacing existing versions',
  (pm) => {
    const config = {
      catalog: { '@vitest/utils': '^4.0.0' },
      catalogs: { tools: { 'vite-plus': '0.3.2', '@vitest/spy': '^4.0.0' } },
    };
    const file = path.join(
      root,
      {
        [PackageManager.pnpm]: 'pnpm-workspace.yaml',
        [PackageManager.yarn]: '.yarnrc.yml',
        [PackageManager.bun]: 'package.json',
      }[pm],
    );
    fs.writeFileSync(file, pm === PackageManager.bun ? JSON.stringify(config) : stringify(config));
    fs.writeFileSync(
      path.join(project, 'package.json'),
      JSON.stringify({
        devDependencies: {
          'vite-plus': 'catalog:tools',
          '@vitest/utils': '^5.0.0',
          '@vitest/spy': 'catalog:tools',
        },
      }),
    );
    rewriteMonorepoProject(
      project,
      pm,
      true,
      true,
      undefined,
      createCatalogDependencyResolver(root, pm),
    );
    ensureCreateCatalogEntries(root, project, pm);
    expect(readJsonFile(path.join(project, 'package.json')).devDependencies).toMatchObject({
      '@vitest/utils': 'catalog:tools',
      '@vitest/spy': 'catalog:tools',
    });
    expect(pm === PackageManager.bun ? readJsonFile(file) : readYamlFile(file)).toMatchObject({
      catalog: config.catalog,
      catalogs: { tools: { ...config.catalogs.tools, '@vitest/utils': VITEST_VERSION } },
    });
    const before = fs.readFileSync(file, 'utf8');
    ensureCreateCatalogEntries(root, project, pm);
    expect(fs.readFileSync(file, 'utf8')).toBe(before);
  },
);

it('only completes referenced install dependencies in the aligned Vitest package set', () => {
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      dependencies: { '@vitest/utils': 'catalog:tools' },
      optionalDependencies: { '@vitest/spy': 'catalog:tools' },
      devDependencies: { '@vitest/expect': 'catalog:tools', '@vitest/mocker': '^5.0.0' },
      peerDependencies: { '@vitest/browser': 'catalog:tools' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.pnpm);
  expect(readYamlFile(workspaceFile)).toEqual({
    packages: ['apps/*'],
    catalogs: { tools: { '@vitest/utils': VITEST_VERSION, '@vitest/spy': VITEST_VERSION } },
  });
});

it('leaves Vitest ecosystem catalogs unmanaged when the Vitest override is absent', () => {
  const version = VITE_PLUS_OVERRIDE_PACKAGES.vitest;
  try {
    delete VITE_PLUS_OVERRIDE_PACKAGES.vitest;
    fs.writeFileSync(
      path.join(project, 'package.json'),
      JSON.stringify({
        devDependencies: { '@vitest/utils': 'catalog:tools' },
      }),
    );
    const before = fs.readFileSync(workspaceFile, 'utf8');
    ensureCreateCatalogEntries(root, project, PackageManager.pnpm);
    expect(fs.readFileSync(workspaceFile, 'utf8')).toBe(before);
  } finally {
    VITE_PLUS_OVERRIDE_PACKAGES.vitest = version;
  }
});

describe.each([PackageManager.yarn, PackageManager.bun])('%s workspace catalogs', (pm) => {
  it.each(['missing', 'partial', 'named', 'default'] as const)(
    'completes a %s catalog without changing existing versions',
    (kind) => {
      const existing = { 'vite-plus': '0.3.2', react: '^19.0.0' };
      const config = {
        missing: {},
        partial: { catalog: existing },
        named: { catalogs: { tools: existing } },
        default: { catalogs: { default: existing } },
      }[kind];
      const file = path.join(root, pm === PackageManager.yarn ? '.yarnrc.yml' : 'package.json');
      const contents = { private: true, workspaces: ['apps/*'], ...config };
      if (pm !== PackageManager.yarn || kind !== 'missing') {
        fs.writeFileSync(
          file,
          pm === PackageManager.yarn ? stringify(config) : JSON.stringify(contents),
        );
      }
      fs.writeFileSync(
        path.join(project, 'package.json'),
        JSON.stringify({ devDependencies: { vite: '^8.0.0' } }),
      );
      rewriteMonorepoProject(
        project,
        pm,
        true,
        true,
        undefined,
        createCatalogDependencyResolver(root, pm),
      );
      ensureCreateCatalogEntries(root, project, pm);
      const spec =
        kind === 'named'
          ? 'catalog:tools'
          : kind === 'default' && pm === PackageManager.yarn
            ? 'catalog:default'
            : 'catalog:';
      expect(readJsonFile(path.join(project, 'package.json')).devDependencies).toEqual({
        vite: spec,
        'vite-plus': spec,
      });
      const result = pm === PackageManager.yarn ? readYamlFile(file) : readJsonFile(file);
      const entries = {
        ...(kind === 'missing' ? { 'vite-plus': VITE_PLUS_VERSION } : existing),
        vite: VITE_PLUS_OVERRIDE_PACKAGES.vite,
      };
      const expected =
        kind === 'named'
          ? { catalogs: { tools: entries } }
          : kind === 'default'
            ? { catalogs: { default: entries } }
            : { catalog: entries };
      expect(result).toEqual(
        pm === PackageManager.yarn
          ? expected
          : { private: true, workspaces: ['apps/*'], ...expected },
      );
      const before = fs.readFileSync(file, 'utf8');
      ensureCreateCatalogEntries(root, project, pm);
      expect(fs.readFileSync(file, 'utf8')).toBe(before);
    },
  );
});

it('fills the active Bun catalog even when ignored root catalogs have those entries', () => {
  const file = path.join(root, 'package.json');
  const ignored = { vite: '0.0.0-unused', 'vite-plus': '0.0.0-unused' };
  fs.writeFileSync(
    file,
    JSON.stringify({
      workspaces: { packages: ['apps/*'], catalog: { react: '^19.0.0' } },
      catalog: ignored,
      catalogs: { tools: ignored },
    }),
  );
  const resolver = createCatalogDependencyResolver(root, PackageManager.bun)!;
  expect(resolver('catalog:', 'vite')).toBeUndefined();
  expect(resolver('catalog:tools', 'vite')).toBeUndefined();
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      devDependencies: { vite: 'catalog:', 'vite-plus': 'catalog:tools' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.bun);
  expect(readJsonFile(file)).toEqual({
    workspaces: {
      packages: ['apps/*'],
      catalog: { react: '^19.0.0', vite: VITE_PLUS_OVERRIDE_PACKAGES.vite },
      catalogs: { tools: { 'vite-plus': VITE_PLUS_VERSION } },
    },
    catalog: ignored,
    catalogs: { tools: ignored },
  });
});

it('keeps Yarn default and named default catalog references distinct', () => {
  fs.writeFileSync(
    path.join(root, '.yarnrc.yml'),
    stringify({
      catalog: { vite: '^8.0.0' },
      catalogs: { default: { 'vite-plus': '0.3.2' } },
    }),
  );
  const resolver = createCatalogDependencyResolver(root, PackageManager.yarn)!;
  expect(resolver.preferredCatalogSpec).toBe('catalog:default');
  expect(resolver('catalog:', 'vite')).toBe('^8.0.0');
  expect(resolver('catalog:', 'vite-plus')).toBeUndefined();
  expect(resolver('catalog:default', 'vite-plus')).toBe('0.3.2');
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      devDependencies: { vite: 'catalog:default', 'vite-plus': 'catalog:default' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.yarn);
  expect(readYamlFile(path.join(root, '.yarnrc.yml'))).toEqual({
    catalog: { vite: '^8.0.0' },
    catalogs: { default: { 'vite-plus': '0.3.2', vite: VITE_PLUS_OVERRIDE_PACKAGES.vite } },
  });
});

it('uses the same named default catalog when migrating Yarn settings', () => {
  const file = path.join(root, '.yarnrc.yml');
  fs.writeFileSync(
    file,
    stringify({
      catalog: { react: '^19.0.0' },
      catalogs: { default: { 'vite-plus': '0.3.2' } },
    }),
  );
  rewriteYarnrcYml(root, false, new Set(), new Set(), true, false);
  const config = readYamlFile(file);
  expect(config.catalog).toEqual({ react: '^19.0.0' });
  expect(config).toMatchObject({
    catalogs: {
      default: { 'vite-plus': VITE_PLUS_VERSION, vite: VITE_PLUS_OVERRIDE_PACKAGES.vite },
    },
  });
  expect(yarnrcSatisfiesVitePlus(root, false, true, false)).toBe(true);
});

it('uses Bun catalogs.default before its catalog shorthand', () => {
  const file = path.join(root, 'package.json');
  fs.writeFileSync(
    file,
    JSON.stringify({
      workspaces: ['apps/*'],
      catalog: { vite: '0.0.0-unused' },
      catalogs: { default: { 'vite-plus': '0.3.2' } },
    }),
  );
  const resolver = createCatalogDependencyResolver(root, PackageManager.bun)!;
  expect(resolver('catalog:', 'vite')).toBeUndefined();
  expect(resolver('catalog:default', 'vite-plus')).toBe('0.3.2');
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      devDependencies: { vite: 'catalog:', 'vite-plus': 'catalog:' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.bun);
  expect(readJsonFile(file)).toEqual({
    workspaces: ['apps/*'],
    catalog: { vite: '0.0.0-unused' },
    catalogs: { default: { 'vite-plus': '0.3.2', vite: VITE_PLUS_OVERRIDE_PACKAGES.vite } },
  });
});

it('fills Bun nested named catalogs without moving or replacing existing settings', () => {
  const file = path.join(root, 'package.json');
  fs.writeFileSync(
    file,
    JSON.stringify({
      private: true,
      workspaces: { packages: ['apps/*'], catalogs: { tools: { 'vite-plus': '0.3.2' } } },
      catalog: { react: '^19.0.0' },
    }),
  );
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      devDependencies: { vite: 'catalog:tools', 'vite-plus': 'catalog:tools' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.bun);
  expect(readJsonFile(file)).toEqual({
    private: true,
    workspaces: {
      packages: ['apps/*'],
      catalogs: {
        tools: {
          'vite-plus': '0.3.2',
          vite: VITE_PLUS_OVERRIDE_PACKAGES.vite,
        },
      },
    },
    catalog: { react: '^19.0.0' },
  });
});

it('keeps Bun nested default catalogs separate from top-level settings', () => {
  const file = path.join(root, 'package.json');
  const contents = {
    private: true,
    workspaces: { packages: ['apps/*'], catalogs: { default: { 'vite-plus': '0.3.2' } } },
    catalog: { react: '^19.0.0' },
  };
  fs.writeFileSync(file, JSON.stringify(contents));
  fs.writeFileSync(
    path.join(project, 'package.json'),
    JSON.stringify({
      devDependencies: { vite: 'catalog:', 'vite-plus': 'catalog:' },
    }),
  );
  ensureCreateCatalogEntries(root, project, PackageManager.bun);
  expect(readJsonFile(file)).toEqual({
    ...contents,
    workspaces: {
      packages: ['apps/*'],
      catalogs: {
        default: {
          'vite-plus': '0.3.2',
          vite: VITE_PLUS_OVERRIDE_PACKAGES.vite,
        },
      },
    },
  });
});

describe('catalogs for applications created in existing pnpm workspaces', () => {
  it('makes references generated by member integration resolvable in a plain workspace', () => {
    const workspace = integrate();
    expect(readJsonFile(path.join(project, 'package.json')).devDependencies).toMatchObject({
      vite: 'catalog:',
      'vite-plus': 'catalog:',
    });
    expect(workspace).toEqual({
      packages: ['apps/*'],
      catalog: { vite: VITE_PLUS_OVERRIDE_PACKAGES.vite, 'vite-plus': VITE_PLUS_VERSION },
    });
  });

  it('adds missing entries to the selected named catalog without changing existing versions', () => {
    fs.writeFileSync(
      workspaceFile,
      `packages: [apps/*]\ncatalogs:\n  tools:\n    vite-plus: ${VITE_PLUS_VERSION}\n  unrelated:\n    react: ^19.0.0\n`,
    );
    expect(integrate()).toEqual({
      packages: ['apps/*'],
      catalogs: {
        tools: { 'vite-plus': VITE_PLUS_VERSION, vite: VITE_PLUS_OVERRIDE_PACKAGES.vite },
        unrelated: { react: '^19.0.0' },
      },
    });
    expect(readJsonFile(path.join(project, 'package.json')).devDependencies).toMatchObject({
      vite: 'catalog:tools',
      'vite-plus': 'catalog:tools',
    });
  });

  it('uses catalogs.default without creating a conflicting top-level catalog', () => {
    fs.writeFileSync(
      workspaceFile,
      'packages: [apps/*]\ncatalogs:\n  default:\n    react: ^19.0.0\n',
    );
    expect(integrate()).toEqual({
      packages: ['apps/*'],
      catalogs: {
        default: {
          react: '^19.0.0',
          vite: VITE_PLUS_OVERRIDE_PACKAGES.vite,
          'vite-plus': VITE_PLUS_VERSION,
        },
      },
    });
  });

  it('leaves a complete workspace file byte-for-byte unchanged', () => {
    const content =
      'packages: [apps/*]\n# User-owned versions\ncatalog:\n  vite: npm:@voidzero-dev/vite-plus-core@0.3.2\n  vite-plus: 0.3.2\noverrides:\n  react: ^19.0.0\n';
    fs.writeFileSync(workspaceFile, content);
    integrate();
    expect(fs.readFileSync(workspaceFile, 'utf8')).toBe(content);
  });

  it('fills the exact catalog named by each dependency reference', () => {
    const workspace = integrate({ vite: 'catalog:build', 'vite-plus': 'catalog:default' });
    expect(workspace).toEqual({
      packages: ['apps/*'],
      catalog: { 'vite-plus': VITE_PLUS_VERSION },
      catalogs: { build: { vite: VITE_PLUS_OVERRIDE_PACKAGES.vite } },
    });
  });

  it('leaves templates without a package manifest alone', () => {
    const content = fs.readFileSync(workspaceFile, 'utf8');
    ensureCreateCatalogEntries(root, project, PackageManager.pnpm);
    expect(fs.readFileSync(workspaceFile, 'utf8')).toBe(content);
  });

  it('does not create catalog entries for concrete dependencies or unknown template references', () => {
    const content = fs.readFileSync(workspaceFile, 'utf8');
    fs.writeFileSync(
      path.join(project, 'package.json'),
      JSON.stringify({
        devDependencies: {
          vite: 'file:../core.tgz',
          'vite-plus': 'file:../cli.tgz',
          custom: 'catalog:',
        },
      }),
    );
    ensureCreateCatalogEntries(root, project, PackageManager.pnpm);
    expect(fs.readFileSync(workspaceFile, 'utf8')).toBe(content);
  });
});
