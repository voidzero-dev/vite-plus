import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { info: vi.fn(), success: vi.fn(), warn: vi.fn() },
}));
vi.mock('../../utils/command.ts', () => ({
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
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-prettier-'));
    fs.writeFileSync(path.join(projectPath, 'package.json'), JSON.stringify(packageJson));
  });

  afterEach(() => {
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
      expect.objectContaining({ args: ['fmt', '--migrate=prettier'], cwd: projectPath }),
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
});
