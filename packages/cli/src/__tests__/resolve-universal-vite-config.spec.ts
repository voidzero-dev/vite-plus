import fs from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';

import { resolveUniversalViteConfig } from '../resolve-vite-config.ts';
import { VITE_CONFIG_FILES } from '../utils/constants.ts';

describe('resolveUniversalViteConfig', () => {
  let root: string;

  beforeEach(() => {
    root = fs.realpathSync(fs.mkdtempSync(path.join(tmpdir(), 'vp-universal-config-')));
    fs.writeFileSync(path.join(root, 'package.json'), '{"type":"module"}');
  });

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true });
    vi.restoreAllMocks();
  });

  it('returns empty metadata without searching ancestors and observes a new config', async () => {
    const cwd = path.join(root, 'app');
    fs.mkdirSync(cwd);
    fs.writeFileSync(path.join(root, 'vite.config.ts'), 'throw new Error("outside workspace");');
    expect(JSON.parse(await resolveUniversalViteConfig(null, cwd))).toEqual({});

    const configFile = path.join(cwd, 'vite.config.ts');
    fs.writeFileSync(configFile, 'export default { check: { fmt: false } };');
    expect(JSON.parse(await resolveUniversalViteConfig(null, cwd))).toMatchObject({
      configFile: configFile.replaceAll('\\', '/'),
      check: { fmt: false },
    });
    fs.unlinkSync(configFile);
    expect(JSON.parse(await resolveUniversalViteConfig(null, cwd))).toEqual({});
  });

  it.each(VITE_CONFIG_FILES)(
    'resolves %s with config functions and plugin hooks',
    async (filename) => {
      const configFile = path.join(root, filename);
      const declaration =
        filename.endsWith('.cjs') || filename.endsWith('.cts')
          ? 'module.exports ='
          : 'export default';
      fs.writeFileSync(
        configFile,
        `${declaration} async ({ command, mode }) => ({
      lint: {}, fmt: {}, run: {}, staged: { '*.ts': 'vp check --fix' },
      plugins: [{ name: 'metadata', config() {
        return { check: { fmt: command !== 'build', lint: mode !== 'development' } };
      } }],
    });`,
      );
      expect(JSON.parse(await resolveUniversalViteConfig(null, root))).toEqual({
        configFile: configFile.replaceAll('\\', '/'),
        lint: {},
        fmt: {},
        run: {},
        staged: { '*.ts': 'vp check --fix' },
        check: { fmt: false, lint: false },
      });
    },
  );

  it('preserves callback and config errors', async () => {
    const error = new Error('callback failed');
    await expect(resolveUniversalViteConfig(error, root)).rejects.toBe(error);
    vi.spyOn(console, 'error').mockImplementation(() => {});
    fs.writeFileSync(path.join(root, 'vite.config.ts'), 'throw new Error("invalid config");');
    await expect(resolveUniversalViteConfig(null, root)).rejects.toThrow('invalid config');
  });
});
