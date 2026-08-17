import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { detectConfigs } from '../detector.ts';

describe('detectConfigs — dynamic Oxc configs', () => {
  let tmpDir: string;

  afterEach(() => {
    if (tmpDir) {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it.each([
    ['oxlint.config.ts', 'oxlintConfig'],
    ['oxlint.config.mts', 'oxlintConfig'],
    ['oxfmt.config.ts', 'oxfmtConfig'],
    ['oxfmt.config.mts', 'oxfmtConfig'],
  ] as const)('detects %s', (filename, configKey) => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-detector-'));
    fs.writeFileSync(path.join(tmpDir, filename), 'export default {};\n');

    expect(detectConfigs(tmpDir)[configKey]).toBe(filename);
  });

  it('prefers JSON configs when both JSON and dynamic configs exist', () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-detector-'));
    fs.writeFileSync(path.join(tmpDir, '.oxlintrc.json'), '{}\n');
    fs.writeFileSync(path.join(tmpDir, 'oxlint.config.ts'), 'export default {};\n');
    fs.writeFileSync(path.join(tmpDir, '.oxfmtrc.jsonc'), '{}\n');
    fs.writeFileSync(path.join(tmpDir, 'oxfmt.config.mts'), 'export default {};\n');

    expect(detectConfigs(tmpDir)).toMatchObject({
      oxlintConfig: '.oxlintrc.json',
      oxfmtConfig: '.oxfmtrc.jsonc',
    });
  });
});
