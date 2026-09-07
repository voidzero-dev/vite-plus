import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { dropDeadOxlintPluginsDependency } from '../migrator.ts';

describe('Oxlint plugin dependency cleanup', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-plugin-dependency-'));
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
  });

  it.each(['devDependencies', 'optionalDependencies'] as const)(
    'retains an import alias target in %s',
    (field) => {
      const pkg = {
        imports: { '#plugin-api': '@oxlint/plugins' },
        [field]: { '@oxlint/plugins': '^1.79.0' },
      };
      const packageJsonPath = path.join(projectPath, 'package.json');
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
      fs.writeFileSync(
        path.join(projectPath, 'plugin.js'),
        `import { defineRule } from '#plugin-api';`,
      );

      dropDeadOxlintPluginsDependency(projectPath);

      expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
    },
  );

  it('retains conditional aliases in nested non-workspace packages', () => {
    const pkg = { devDependencies: { '@oxlint/plugins': '^1.79.0' } };
    const packageJsonPath = path.join(projectPath, 'package.json');
    fs.writeFileSync(packageJsonPath, JSON.stringify(pkg));
    const examplePath = path.join(projectPath, 'example');
    fs.mkdirSync(examplePath);
    fs.writeFileSync(
      path.join(examplePath, 'package.json'),
      JSON.stringify({
        imports: {
          '#plugin-api': {
            node: { import: '@oxlint/plugins', require: '@oxlint/plugins' },
            default: './fallback.js',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(examplePath, 'plugin.js'),
      `import { defineRule } from '#plugin-api';`,
    );

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual(pkg);
  });

  it('removes unused dependencies when aliases already target vite-plus', () => {
    const packageJsonPath = path.join(projectPath, 'package.json');
    const imports = { '#plugin-api': 'vite-plus/lint/plugins' };
    fs.writeFileSync(
      packageJsonPath,
      JSON.stringify({
        imports,
        devDependencies: { '@oxlint/plugins': '^1.79.0', 'vite-plus': 'latest' },
        optionalDependencies: { '@oxlint/plugins': '^1.79.0' },
      }),
    );
    fs.writeFileSync(
      path.join(projectPath, 'plugin.js'),
      `import { defineRule } from '#plugin-api';`,
    );

    dropDeadOxlintPluginsDependency(projectPath);

    expect(JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))).toEqual({
      imports,
      devDependencies: { 'vite-plus': 'latest' },
      optionalDependencies: {},
    });
  });
});
