import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { info: vi.fn(), success: vi.fn(), warn: vi.fn() },
}));
vi.mock(import('../../utils/command.ts'), async (importOriginal) => ({
  ...(await importOriginal()),
  runCommandSilently: vi.fn(),
}));

import { runCommandSilently } from '../../utils/command.ts';
import { migratePrettierToOxfmt } from '../migrator/prettier.ts';
import { createMigrationReport } from '../report.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

describe('Prettier migration', () => {
  let projectPath: string;
  const packageJson = {
    name: 'fixture',
    scripts: { format: 'prettier --write .' },
    devDependencies: { prettier: '^3.0.0', 'prettier-plugin-tailwindcss': '^0.7.0' },
    'lint-staged': { '*.ts': 'prettier --write' },
  };

  beforeEach(() => {
    vi.stubEnv('VP_CLI_BIN', undefined);
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-prettier-'));
    fs.writeFileSync(path.join(projectPath, 'package.json'), JSON.stringify(packageJson));
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    fs.rmSync(projectPath, { recursive: true, force: true });
    mockRunCommandSilently.mockReset();
  });

  it.each([
    ['.prettierrc.yml', 'semi: false\n', 'YAML, JSON5 and TOML formats are not supported.'],
    ['.prettierrc.yaml', 'semi: false\n', 'YAML, JSON5 and TOML formats are not supported.'],
    ['.prettierrc.toml', 'semi = false\n', 'YAML, JSON5 and TOML formats are not supported.'],
    ['.prettierrc.json5', '{ semi: false }\n', 'YAML, JSON5 and TOML formats are not supported.'],
    ['.prettierrc', 'semi: false\n', 'Only JSON format is supported for `.prettierrc`.'],
  ])('keeps Prettier when Oxfmt cannot load %s', async (configFile, config, diagnostic) => {
    fs.writeFileSync(path.join(projectPath, configFile), config);
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 1,
      stdout: Buffer.alloc(0),
      stderr: Buffer.from(`Failed to load Prettier config at: ${configFile}\n${diagnostic}`),
    });
    const report = createMigrationReport();

    await expect(
      migratePrettierToOxfmt(projectPath, false, configFile, undefined, { silent: true, report }),
    ).resolves.toBe('skipped');

    expect(mockRunCommandSilently).toHaveBeenCalledWith(
      expect.objectContaining({
        args: [expect.any(String), 'fmt', '--migrate=prettier'],
        cwd: projectPath,
      }),
    );
    expect(fs.readFileSync(path.join(projectPath, configFile), 'utf8')).toBe(config);
    expect(fs.readFileSync(path.join(projectPath, 'package.json'), 'utf8')).toBe(
      JSON.stringify(packageJson),
    );
    expect(fs.existsSync(path.join(projectPath, '.oxfmtrc.json'))).toBe(false);
    expect(report.prettierMigrated).toBe(false);
    expect(report.removedConfigCount).toBe(0);
  });

  it('still fails on other Oxfmt errors and keeps Prettier', async () => {
    const config = '{ invalid json }';
    fs.writeFileSync(path.join(projectPath, '.prettierrc.json'), config);
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 1,
      stdout: Buffer.alloc(0),
      stderr: Buffer.from('Failed to load Prettier config at: .prettierrc.json\nInvalid JSON'),
    });

    await expect(
      migratePrettierToOxfmt(projectPath, false, '.prettierrc.json', undefined, { silent: true }),
    ).resolves.toBe('failed');

    expect(fs.readFileSync(path.join(projectPath, '.prettierrc.json'), 'utf8')).toBe(config);
    expect(fs.readFileSync(path.join(projectPath, 'package.json'), 'utf8')).toBe(
      JSON.stringify(packageJson),
    );
  });

  it('still fails when Oxfmt cannot start', async () => {
    mockRunCommandSilently.mockRejectedValue(new Error('spawn ENOENT'));

    await expect(
      migratePrettierToOxfmt(projectPath, false, '.prettierrc.json', undefined, { silent: true }),
    ).resolves.toBe('failed');
  });

  it('removes Prettier only after Oxfmt succeeds', async () => {
    fs.writeFileSync(path.join(projectPath, '.prettierrc.json'), '{"semi":false}');
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });
    const report = createMigrationReport();

    await expect(
      migratePrettierToOxfmt(projectPath, false, '.prettierrc.json', undefined, {
        silent: true,
        report,
      }),
    ).resolves.toBe('migrated');

    expect(fs.existsSync(path.join(projectPath, '.prettierrc.json'))).toBe(false);
    expect(JSON.parse(fs.readFileSync(path.join(projectPath, 'package.json'), 'utf8'))).toEqual({
      name: 'fixture',
      scripts: { format: 'vp fmt .' },
      devDependencies: {},
      'lint-staged': { '*.ts': 'vp fmt' },
    });
    expect(report.prettierMigrated).toBe(true);
  });

  // `vp fmt --migrate` converts nothing while the config declares `fmt`, and the
  // first `vp migrate` injects `fmt: {}` even when Prettier stays.
  it.each([
    [0, 'migrated'],
    [1, 'failed'],
  ] as const)(
    'clears the injected empty fmt block for the conversion (exit code %s)',
    async (exitCode, outcome) => {
      fs.writeFileSync(path.join(projectPath, '.prettierrc.json'), '{"semi":false}');
      const viteConfigPath = path.join(projectPath, 'vite.config.ts');
      const viteConfig = `import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: { rules: {} },
});
`;
      fs.writeFileSync(viteConfigPath, viteConfig);
      let configDuringConversion = '';
      mockRunCommandSilently.mockImplementation(async () => {
        configDuringConversion = fs.readFileSync(viteConfigPath, 'utf8');
        return { exitCode, stdout: Buffer.alloc(0), stderr: Buffer.from('Invalid JSON') };
      });

      await expect(
        migratePrettierToOxfmt(projectPath, false, '.prettierrc.json', undefined, { silent: true }),
      ).resolves.toBe(outcome);

      expect(configDuringConversion).not.toContain('fmt');
      expect(configDuringConversion).toContain('lint: { rules: {} }');
      // A failed conversion leaves the project as it was.
      expect(fs.readFileSync(viteConfigPath, 'utf8')).toBe(
        exitCode === 0 ? configDuringConversion : viteConfig,
      );
      expect(fs.existsSync(path.join(projectPath, '.prettierrc.json'))).toBe(exitCode !== 0);
    },
  );

  it.each([
    ['has user settings', 'fmt: { singleQuote: true },'],
    // Vite+ prepends its empty block. One after a spread overrides the spread,
    // and the converted options would land before it and lose to it.
    ['follows a spread', '...{ fmt: { semi: true } },\n  fmt: {},'],
  ])('keeps Prettier when the fmt block %s', async (_name, properties) => {
    fs.writeFileSync(path.join(projectPath, '.prettierrc.json'), '{"semi":false}');
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    const viteConfig = `import { defineConfig } from 'vite-plus';

export default defineConfig({
  ${properties}
});
`;
    fs.writeFileSync(viteConfigPath, viteConfig);
    const report = createMigrationReport();

    await expect(
      migratePrettierToOxfmt(projectPath, false, '.prettierrc.json', undefined, {
        silent: true,
        report,
      }),
    ).resolves.toBe('skipped');

    // The conversion would be skipped, so removing Prettier would lose its options.
    expect(mockRunCommandSilently).not.toHaveBeenCalled();
    expect(fs.readFileSync(viteConfigPath, 'utf8')).toBe(viteConfig);
    expect(fs.readFileSync(path.join(projectPath, '.prettierrc.json'), 'utf8')).toBe(
      '{"semi":false}',
    );
    expect(fs.readFileSync(path.join(projectPath, 'package.json'), 'utf8')).toBe(
      JSON.stringify(packageJson),
    );
    expect(report.prettierMigrated).toBe(false);
    expect(report.warnings).toEqual([
      'Kept Prettier because vite.config.ts already has a fmt config. Move the Prettier options into it manually, then remove Prettier.',
    ]);
  });
});
