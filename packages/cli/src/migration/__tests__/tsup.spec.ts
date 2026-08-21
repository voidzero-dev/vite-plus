import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mockConfirm, mockInfo, mockWarn } = vi.hoisted(() => ({
  mockConfirm: vi.fn(),
  mockInfo: vi.fn(),
  mockWarn: vi.fn(),
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  confirm: mockConfirm,
  isCancel: () => false,
  log: {
    info: mockInfo,
    success: vi.fn(),
    warn: mockWarn,
  },
}));

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn(),
}));
vi.mock('../../utils/prompts.ts', () => ({
  cancelAndExit: vi.fn(),
}));

import { PackageManager } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { TSDOWN_MIGRATE_VERSION, TSDOWN_MIGRATION_SKILL_URL } from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { confirmTsupMigration, detectTsupProject, migrateTsupToTsdown } from '../migrator/tsup.ts';
import { createMigrationReport } from '../report.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

function manualMigrationOptions(targetLabel = 'the project root'): string {
  return [
    'Choose one of these manual migration methods:',
    `  1. Run \`vp dlx tsdown-migrate\` in ${targetLabel}.`,
    '  2. Use the tsdown migration skill:',
    `     ${TSDOWN_MIGRATION_SKILL_URL}`,
  ].join('\n');
}

function migrationSkillGuidance(instructions: string[]): string {
  return [
    ...instructions,
    '',
    'Use the tsdown migration skill for guidance:',
    `  ${TSDOWN_MIGRATION_SKILL_URL}`,
  ].join('\n');
}

describe('tsup migration', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsup-'));
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      `${JSON.stringify({ name: 'fixture', devDependencies: { tsup: '^8.5.0' } }, null, 2)}\n`,
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsup.config.ts'),
      "import { defineConfig } from 'tsup';\nexport default defineConfig({ dts: true });\n",
    );
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });
    mockConfirm.mockResolvedValue(true);
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
    mockRunCommandSilently.mockReset();
    mockConfirm.mockReset();
    mockInfo.mockReset();
    mockWarn.mockReset();
  });

  it('passes the package manager as a separate CLI argument', async () => {
    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(true);

    expect(mockRunCommandSilently).toHaveBeenCalledWith(
      expect.objectContaining({
        args: [
          'dlx',
          `tsdown-migrate@${TSDOWN_MIGRATE_VERSION}`,
          '--yes',
          '--package-manager',
          'npm',
          '--no-install',
        ],
      }),
    );
  });

  it('refuses to overwrite an existing tsdown config', async () => {
    const tsupConfigPath = path.join(projectPath, 'tsup.config.ts');
    const tsdownConfigPath = path.join(projectPath, 'tsdown.config.ts');
    const originalTsupConfig = fs.readFileSync(tsupConfigPath, 'utf8');
    const originalTsdownConfig = 'export default { existing: true };\n';
    fs.writeFileSync(tsdownConfigPath, originalTsdownConfig);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(fs.readFileSync(tsupConfigPath, 'utf8')).toBe(originalTsupConfig);
    expect(fs.readFileSync(tsdownConfigPath, 'utf8')).toBe(originalTsdownConfig);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these tsdown configs already exist:\n  tsdown.config.ts',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve this configuration conflict manually:',
        '  1. Merge the tsup and tsdown configurations into `pack` in `vite.config.*`.',
        '  2. Do not run `tsdown-migrate`. It can overwrite the existing tsdown configuration.',
      ]),
    );
  });

  it('refuses to overwrite an inline tsdown config', async () => {
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packageJsonPath = path.join(projectPath, 'package.json');
    const originalPackageJson = {
      name: 'fixture',
      scripts: { build: 'tsup' },
      devDependencies: { tsup: '^8.5.0' },
      tsup: { entry: ['src/index.ts'] },
      tsdown: { entry: ['src/existing.ts'] },
    };
    fs.writeFileSync(packageJsonPath, `${JSON.stringify(originalPackageJson, null, 2)}\n`);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'package.json#tsup', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(readJsonFile(packageJsonPath)).toEqual(originalPackageJson);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these tsdown configs already exist:\n  package.json#tsdown',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve this configuration conflict manually:',
        '  1. Merge the tsup and tsdown configurations into `pack` in `vite.config.*`.',
        '  2. Do not run `tsdown-migrate`. It can overwrite the existing tsdown configuration.',
      ]),
    );
  });

  it('refuses to migrate an inline tsup config', async () => {
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packageJsonPath = path.join(projectPath, 'package.json');
    const originalPackageJson = {
      name: 'fixture',
      scripts: { build: 'tsup' },
      devDependencies: { tsup: '^8.5.0' },
      tsup: { entry: ['src/index.ts'] },
    };
    fs.writeFileSync(packageJsonPath, `${JSON.stringify(originalPackageJson, null, 2)}\n`);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'package.json#tsup', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(readJsonFile(packageJsonPath)).toEqual(originalPackageJson);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these inline tsup configs cannot be migrated automatically:\n  package.json#tsup',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve this inline configuration manually:',
        '  1. Move each `package.json#tsup` configuration into `pack` in `vite.config.*`.',
        '  2. Do not run `tsdown-migrate`. Vite+ Pack does not read `package.json#tsdown`.',
      ]),
    );
  });

  it('refuses to migrate a script that uses a custom tsup config', async () => {
    const packageJsonPath = path.join(projectPath, 'package.json');
    const originalPackageJson = {
      name: 'fixture',
      scripts: { build: 'tsup --config configs/legacy.ts' },
      devDependencies: { tsup: '^8.5.0' },
    };
    fs.mkdirSync(path.join(projectPath, 'configs'));
    fs.writeFileSync(path.join(projectPath, 'configs/legacy.ts'), 'export default {};\n');
    fs.writeFileSync(packageJsonPath, `${JSON.stringify(originalPackageJson, null, 2)}\n`);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(readJsonFile(packageJsonPath)).toEqual(originalPackageJson);
    expect(fs.existsSync(path.join(projectPath, 'tsup.config.ts'))).toBe(true);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these scripts use configs that cannot be migrated automatically:\n  package.json#build -> configs/legacy.ts',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve these config paths manually:',
        '  1. Migrate each listed config into `pack` in `vite.config.*`.',
        '  2. Update each listed script.',
        '  3. Do not run `tsdown-migrate`. It cannot safely resolve these config paths.',
      ]),
    );
  });

  it('refuses a default config path that cleanup cannot remove', async () => {
    const packageJsonPath = path.join(projectPath, 'package.json');
    const originalPackageJson = {
      name: 'fixture',
      scripts: { build: 'tsup --config ././tsup.config.ts' },
      devDependencies: { tsup: '^8.5.0' },
    };
    fs.writeFileSync(packageJsonPath, `${JSON.stringify(originalPackageJson, null, 2)}\n`);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(readJsonFile(packageJsonPath)).toEqual(originalPackageJson);
    expect(fs.existsSync(path.join(projectPath, 'tsup.config.ts'))).toBe(true);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these scripts use configs that cannot be migrated automatically:\n  package.json#build -> ././tsup.config.ts',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve these config paths manually:',
        '  1. Migrate each listed config into `pack` in `vite.config.*`.',
        '  2. Update each listed script.',
        '  3. Do not run `tsdown-migrate`. It cannot safely resolve these config paths.',
      ]),
    );
  });

  it('refuses to remove selectors for multiple standard tsup configs', async () => {
    fs.writeFileSync(path.join(projectPath, 'tsup.config.js'), 'export default {};\n');
    const packageJsonPath = path.join(projectPath, 'package.json');
    const originalPackageJson = {
      name: 'fixture',
      scripts: {
        buildTs: 'tsup --config tsup.config.ts',
        buildJs: 'tsup --config tsup.config.js',
      },
      devDependencies: { tsup: '^8.5.0' },
    };
    fs.writeFileSync(packageJsonPath, `${JSON.stringify(originalPackageJson, null, 2)}\n`);

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(readJsonFile(packageJsonPath)).toEqual(originalPackageJson);
    expect(mockWarn).toHaveBeenCalledWith(
      'Automatic tsup migration was skipped because these scripts use configs that cannot be migrated automatically:\n  package.json#buildJs -> tsup.config.js',
    );
    expect(mockInfo).toHaveBeenCalledWith(
      migrationSkillGuidance([
        'Resolve these config paths manually:',
        '  1. Migrate each listed config into `pack` in `vite.config.*`.',
        '  2. Update each listed script.',
        '  3. Do not run `tsdown-migrate`. It cannot safely resolve these config paths.',
      ]),
    );
  });

  it('detects a workspace-only tsup config', () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packagePath = path.join(projectPath, 'packages/a');
    fs.mkdirSync(packagePath, { recursive: true });
    fs.writeFileSync(
      path.join(packagePath, 'package.json'),
      '{"name":"a","devDependencies":{"tsup":"^8.5.0"}}\n',
    );
    fs.writeFileSync(path.join(packagePath, 'tsup.config.ts'), 'export default {};\n');

    expect(detectTsupProject(projectPath, [{ name: 'a', path: 'packages/a' }])).toEqual({
      hasDependency: true,
      hasConfig: true,
      configFile: undefined,
    });
  });

  it('restores all workspace targets when a later migration fails', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    const originalFiles = new Map<string, string>();

    for (const workspacePackage of packages) {
      const packagePath = path.join(projectPath, workspacePackage.path);
      const packageJson = `${JSON.stringify(
        {
          name: workspacePackage.name,
          scripts: { build: 'tsup' },
          devDependencies: { tsup: '^8.5.0' },
        },
        null,
        2,
      )}\n`;
      const tsupConfig = `export default { name: '${workspacePackage.name}' };\n`;
      fs.mkdirSync(packagePath, { recursive: true });
      fs.writeFileSync(path.join(packagePath, 'package.json'), packageJson);
      fs.writeFileSync(path.join(packagePath, 'tsup.config.ts'), tsupConfig);
      originalFiles.set(path.join(packagePath, 'package.json'), packageJson);
      originalFiles.set(path.join(packagePath, 'tsup.config.ts'), tsupConfig);
    }

    mockRunCommandSilently.mockImplementation(async ({ cwd }) => {
      const packageJsonPath = path.join(cwd, 'package.json');
      fs.writeFileSync(packageJsonPath, '{"name":"partially-migrated"}\n');
      fs.writeFileSync(path.join(cwd, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(cwd, 'tsup.config.ts'));
      return {
        exitCode: path.basename(cwd) === 'b' ? 1 : 0,
        stdout: Buffer.alloc(0),
        stderr: Buffer.alloc(0),
      };
    });

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, undefined, packages, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).toHaveBeenCalledTimes(2);
    for (const [filePath, contents] of originalFiles) {
      expect(fs.readFileSync(filePath, 'utf8')).toBe(contents);
      expect(fs.existsSync(path.join(path.dirname(filePath), 'tsdown.config.ts'))).toBe(false);
    }
  });

  it('preserves workspace packages that do not have a tsup config', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    for (const workspacePackage of packages) {
      const packagePath = path.join(projectPath, workspacePackage.path);
      fs.mkdirSync(packagePath, { recursive: true });
      fs.writeFileSync(
        path.join(packagePath, 'package.json'),
        `${JSON.stringify(
          {
            name: workspacePackage.name,
            scripts: { build: 'tsup' },
            devDependencies: { tsup: '^8.5.0' },
          },
          null,
          2,
        )}\n`,
      );
    }
    fs.writeFileSync(path.join(projectPath, 'packages/a/tsup.config.ts'), 'export default {};\n');
    const packageBPath = path.join(projectPath, 'packages/b/package.json');
    const originalPackageB = fs.readFileSync(packageBPath, 'utf8');

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, undefined, packages, {
        silent: true,
      }),
    ).resolves.toBe(true);

    expect(mockRunCommandSilently).toHaveBeenCalledTimes(1);
    expect(mockRunCommandSilently).toHaveBeenCalledWith(
      expect.objectContaining({ cwd: path.join(projectPath, 'packages/a') }),
    );
    expect(fs.readFileSync(packageBPath, 'utf8')).toBe(originalPackageB);
  });

  it('preserves a root config shared by multiple workspace packages', async () => {
    const originalRootConfig = fs.readFileSync(path.join(projectPath, 'tsup.config.ts'), 'utf8');
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    for (const workspacePackage of packages) {
      const packagePath = path.join(projectPath, workspacePackage.path);
      fs.mkdirSync(packagePath, { recursive: true });
      fs.writeFileSync(
        path.join(packagePath, 'package.json'),
        `${JSON.stringify(
          {
            name: workspacePackage.name,
            scripts: { build: 'tsup --config ../../tsup.config.ts' },
            devDependencies: { tsup: '^8.5.0' },
          },
          null,
          2,
        )}\n`,
      );
    }
    mockRunCommandSilently.mockImplementation(async ({ cwd }) => {
      const packageJsonPath = path.join(cwd, 'package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      packageJson.devDependencies.tsdown = '0.22.14';
      delete packageJson.devDependencies.tsup;
      fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
      fs.writeFileSync(path.join(cwd, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(cwd, 'tsup.config.ts'));
      return { exitCode: 0, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    });
    const report = createMigrationReport();

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, 'tsup.config.ts', packages, {
        silent: true,
        report,
      }),
    ).resolves.toBe(true);

    expect(mockRunCommandSilently).toHaveBeenCalledTimes(1);
    expect(fs.readFileSync(path.join(projectPath, 'tsup.config.ts'), 'utf8')).toBe(
      originalRootConfig,
    );
    expect(fs.existsSync(path.join(projectPath, 'tsdown.config.ts'))).toBe(true);
    expect(readJsonFile(path.join(projectPath, 'package.json')).devDependencies).toMatchObject({
      tsup: '^8.5.0',
      tsdown: '0.22.14',
    });
    for (const workspacePackage of packages) {
      expect(
        readJsonFile(path.join(projectPath, workspacePackage.path, 'package.json')),
      ).toMatchObject({
        scripts: { build: 'tsup --config ../../tsup.config.ts' },
        devDependencies: { tsup: '^8.5.0' },
      });
    }
    expect(report.warnings).toContain(
      'tsup.config.ts is shared by packages/a, packages/b. It was preserved and must be migrated manually.',
    );
  });

  it('preserves a workspace config consumed by the root package', async () => {
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      `${JSON.stringify(
        {
          name: 'workspace',
          private: true,
          scripts: { build: 'tsup --config packages/a/tsup.config.ts' },
          devDependencies: { tsup: '^8.5.0' },
        },
        null,
        2,
      )}\n`,
    );
    const packagePath = path.join(projectPath, 'packages/a');
    fs.mkdirSync(packagePath, { recursive: true });
    fs.writeFileSync(
      path.join(packagePath, 'package.json'),
      '{"name":"a","devDependencies":{"tsup":"^8.5.0"}}\n',
    );
    const originalConfig = 'export default { entry: ["src/index.ts"] };\n';
    fs.writeFileSync(path.join(packagePath, 'tsup.config.ts'), originalConfig);
    mockRunCommandSilently.mockImplementation(async ({ cwd }) => {
      const packageJsonPath = path.join(cwd, 'package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      packageJson.devDependencies.tsdown = '0.22.14';
      delete packageJson.devDependencies.tsup;
      fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
      fs.writeFileSync(path.join(cwd, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(cwd, 'tsup.config.ts'));
      return { exitCode: 0, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    });
    const report = createMigrationReport();

    await expect(
      migrateTsupToTsdown(
        projectPath,
        false,
        PackageManager.pnpm,
        undefined,
        [{ name: 'a', path: 'packages/a' }],
        { silent: true, report },
      ),
    ).resolves.toBe(true);

    expect(fs.readFileSync(path.join(packagePath, 'tsup.config.ts'), 'utf8')).toBe(originalConfig);
    expect(fs.existsSync(path.join(packagePath, 'tsdown.config.ts'))).toBe(true);
    expect(readJsonFile(path.join(projectPath, 'package.json'))).toMatchObject({
      scripts: { build: 'tsup --config packages/a/tsup.config.ts' },
      devDependencies: { tsup: '^8.5.0' },
    });
    expect(readJsonFile(path.join(packagePath, 'package.json')).devDependencies).toMatchObject({
      tsup: '^8.5.0',
      tsdown: '0.22.14',
    });
    expect(report.warnings).toContain(
      'packages/a/tsup.config.ts is shared by the project root. It was preserved and must be migrated manually.',
    );
  });

  it('preserves shared usage when a migration target consumes another target config', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    for (const workspacePackage of packages) {
      const packagePath = path.join(projectPath, workspacePackage.path);
      fs.mkdirSync(packagePath, { recursive: true });
      fs.writeFileSync(
        path.join(packagePath, 'package.json'),
        `${JSON.stringify(
          {
            name: workspacePackage.name,
            scripts:
              workspacePackage.name === 'a'
                ? { local: 'tsup', shared: 'tsup --config ../b/tsup.config.ts' }
                : { local: 'tsup' },
            devDependencies: { tsup: '^8.5.0' },
          },
          null,
          2,
        )}\n`,
      );
      fs.writeFileSync(
        path.join(packagePath, 'tsup.config.ts'),
        `export default { name: '${workspacePackage.name}' };\n`,
      );
    }
    mockRunCommandSilently.mockImplementation(async ({ cwd }) => {
      const packageJsonPath = path.join(cwd, 'package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      for (const scriptName of Object.keys(packageJson.scripts)) {
        packageJson.scripts[scriptName] = packageJson.scripts[scriptName].replaceAll(
          'tsup',
          'tsdown',
        );
      }
      packageJson.devDependencies.tsdown = '0.22.14';
      delete packageJson.devDependencies.tsup;
      fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
      fs.writeFileSync(path.join(cwd, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(cwd, 'tsup.config.ts'));
      return { exitCode: 0, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    });
    const report = createMigrationReport();

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, undefined, packages, {
        silent: true,
        report,
      }),
    ).resolves.toBe(true);

    expect(readJsonFile(path.join(projectPath, 'packages/a/package.json'))).toMatchObject({
      scripts: { local: 'tsdown', shared: 'tsup --config ../b/tsup.config.ts' },
      devDependencies: { tsup: '^8.5.0', tsdown: '0.22.14' },
    });
    expect(fs.existsSync(path.join(projectPath, 'packages/a/tsup.config.ts'))).toBe(false);
    expect(fs.existsSync(path.join(projectPath, 'packages/a/tsdown.config.ts'))).toBe(true);
    expect(readJsonFile(path.join(projectPath, 'packages/b/package.json'))).toMatchObject({
      scripts: { local: 'tsdown' },
      devDependencies: { tsup: '^8.5.0', tsdown: '0.22.14' },
    });
    expect(fs.existsSync(path.join(projectPath, 'packages/b/tsup.config.ts'))).toBe(true);
    expect(fs.existsSync(path.join(projectPath, 'packages/b/tsdown.config.ts'))).toBe(true);
    expect(report.warnings).toContain(
      'packages/b/tsup.config.ts is shared by packages/a. It was preserved and must be migrated manually.',
    );
  });

  it('removes explicit standard tsup config options after migration', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      `${JSON.stringify(
        {
          name: 'fixture',
          scripts: {
            build: 'tsup --config ./tsup.config.ts',
            watch: 'tsup --watch --config=tsup.config.ts',
            wrapped: 'cross-env NODE_ENV=test tsup -c "tsup.config.ts" --watch',
            quotedData: "echo 'tsdown'",
          },
          devDependencies: { tsup: '^8.5.0' },
        },
        null,
        2,
      )}\n`,
    );
    mockRunCommandSilently.mockImplementation(async () => {
      const packageJsonPath = path.join(projectPath, 'package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      for (const name of Object.keys(packageJson.scripts)) {
        packageJson.scripts[name] = packageJson.scripts[name].replace('tsup', 'tsdown');
      }
      packageJson.devDependencies.tsdown = '0.22.14';
      delete packageJson.devDependencies.tsup;
      fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
      fs.writeFileSync(path.join(projectPath, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
      return { exitCode: 0, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    });

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(true);

    const packageJson = JSON.parse(fs.readFileSync(path.join(projectPath, 'package.json'), 'utf8'));
    expect(packageJson.scripts).toEqual({
      build: 'tsdown',
      watch: 'tsdown --watch',
      wrapped: 'cross-env NODE_ENV=test tsdown --watch',
      quotedData: "echo 'tsdown'",
    });
  });

  it('adds successful tsdown-migrate warnings to the migration report', async () => {
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.from('\n WARN  The plugins option requires manual migration.\n'),
      stderr: Buffer.from(
        'Progress: resolved 1\n\n WARN  The splitting option is currently unsupported in tsdown.\n',
      ),
    });
    const report = createMigrationReport();

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, 'tsup.config.ts', undefined, {
        silent: true,
        report,
      }),
    ).resolves.toBe(true);

    expect(report.warnings).toEqual([
      'tsdown-migrate: The plugins option requires manual migration.',
      'tsdown-migrate: The splitting option is currently unsupported in tsdown.',
    ]);
  });

  it('shows the migration skill when automatic migration is declined', async () => {
    mockConfirm.mockResolvedValue(false);

    await expect(confirmTsupMigration(true)).resolves.toBe(false);

    expect(mockInfo).toHaveBeenCalledWith(manualMigrationOptions());
  });

  it('shows the migration skill when automatic migration fails', async () => {
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 1,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockInfo).toHaveBeenCalledWith(
      `Automatic tsup migration failed.\n\n${manualMigrationOptions()}\n`,
    );
  });
});
