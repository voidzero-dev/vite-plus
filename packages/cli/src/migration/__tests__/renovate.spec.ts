import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { addVitePlusRenovatePreset, findRenovateConfigToUpdate } from '../renovate.ts';

describe('Renovate configuration', () => {
  let root: string;

  beforeEach(async () => {
    root = await fs.mkdtemp(path.join(os.tmpdir(), 'vp-renovate-'));
  });

  afterEach(async () => {
    await fs.rm(root, { recursive: true, force: true });
  });

  async function config(name: string, source: string): Promise<string> {
    const file = path.join(root, name);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, source);
    return file;
  }

  async function expectSkipped(file: string, source: string): Promise<void> {
    expect(await findRenovateConfigToUpdate(root)).toBeUndefined();
    expect(await addVitePlusRenovatePreset(file)).toBe(false);
    expect(await fs.readFile(file, 'utf8')).toBe(source);
  }

  it('edits only the first config and preserves comments and existing presets', async () => {
    const source = [
      '{',
      '  // Keep our preset first',
      '  "extends": ["local>org/shared"],',
      '  "labels": ["deps"],',
      '}',
      '',
    ].join('\r\n');
    const active = await config('.github/renovate.jsonc', source);
    const ignored = await config('.renovaterc', '{}\n');
    expect(await findRenovateConfigToUpdate(root)).toBe(active);
    expect(await addVitePlusRenovatePreset(active)).toBe(true);
    expect(await fs.readFile(active, 'utf8')).toBe(
      source.replace('"local>org/shared"', '"local>org/shared","group:vitePlus"'),
    );
    expect(await fs.readFile(ignored, 'utf8')).toBe('{}\n');
    expect(await findRenovateConfigToUpdate(root)).toBeUndefined();
    expect(await addVitePlusRenovatePreset(active)).toBe(false);
  });

  it('preserves JSON5 comments, quotes, indentation, and trailing commas', async () => {
    const source = [
      '// 项目 🚀',
      '{',
      '\tprConcurrentLimit: 0x10,',
      "\tdescription: '\\\\u{61}',",
      '\textends: [',
      "\t\t'config:recommended', // Keep this rule first",
      '\t],',
      '}',
      '',
    ].join('\r\n');
    const file = await config('.gitlab/renovate.json5', source);
    expect(await findRenovateConfigToUpdate(root)).toBe(file);
    expect(await addVitePlusRenovatePreset(file)).toBe(true);
    expect(await fs.readFile(file, 'utf8')).toBe(
      source.replace('\t],', "\t\t'group:vitePlus',\r\n\t],"),
    );
    expect(await addVitePlusRenovatePreset(file)).toBe(false);
  });

  it('inserts a missing comma before a trailing line comment', async () => {
    const source = "{extends: [\n  'first' // keep this comma, too\n]}";
    const file = await config('renovate.json5', source);
    expect(await addVitePlusRenovatePreset(file)).toBe(true);
    expect(await fs.readFile(file, 'utf8')).toBe(
      "{extends: [\n  'first', // keep this comma, too\n  'group:vitePlus'\n]}",
    );
  });

  it.each([
    ['{extends: []}', "{extends: ['group:vitePlus']}"],
    [
      "{labels: ['deps'], enabled: true}",
      "{labels: ['deps'], enabled: true, extends: ['group:vitePlus']}",
    ],
  ])('adds the first preset in %s', async (source, expected) => {
    const file = await config('.renovaterc.json5', source);
    expect(await findRenovateConfigToUpdate(root)).toBe(file);
    expect(await addVitePlusRenovatePreset(file)).toBe(true);
    expect(await fs.readFile(file, 'utf8')).toBe(expected);
    expect(await findRenovateConfigToUpdate(root)).toBeUndefined();
  });

  it.each([
    '{extends: getPresets()}',
    "{extends: ['first'], rules: {...base}}",
    "{extends: ['first'], [key]: true}",
    "{extends: ['first'], limit: 1_000}",
    '{extends: [], limit: 0b10}',
    '{extends: [], enabled: (true)}',
    String.raw`{extends: ['\u{61}']}`,
    "{extends: ['first'], extends: []}",
    "{extends: ['group:vitePlus']}",
    "{extends: ['first'",
  ])('skips unsupported or ineligible JSON5 configs: %s', async (source) => {
    expect.hasAssertions();
    const file = await config('.gitlab/renovate.json5', source);
    await expectSkipped(file, source);
  });

  it('accepts JSON5-only syntax only in a .json5 file', async () => {
    expect.hasAssertions();
    const source = "{extends: ['config:recommended']}";
    const file = await config('renovate.jsonc', source);
    await expectSkipped(file, source);
  });

  it.each(['renovate.jsonc', 'renovate.json5'])(
    'preserves a multiline JSONC preset list in %s',
    async (name) => {
      const source = [
        '{',
        '  "extends": [',
        '    // Organization defaults',
        '    "local>org/shared",',
        '  ],',
        '}',
        '',
      ].join('\n');
      const file = await config(name, source);
      expect(await findRenovateConfigToUpdate(root)).toBe(file);
      expect(await addVitePlusRenovatePreset(file)).toBe(true);
      expect(await fs.readFile(file, 'utf8')).toBe(
        source.replace('    "local>org/shared",', '    "local>org/shared",\n    "group:vitePlus",'),
      );
    },
  );

  it('adds extends to an existing package.json renovate object after other migration edits', async () => {
    const file = await config('package.json', '{"name":"old","renovate":{}}\n');
    expect(await findRenovateConfigToUpdate(root)).toBe(file);
    await fs.writeFile(
      file,
      '{"name":"new","dependencies":{"constructor":"1"},"renovate":{"labels":["deps"]}}\n',
    );
    await addVitePlusRenovatePreset(file);
    expect(JSON.parse(await fs.readFile(file, 'utf8'))).toEqual({
      name: 'new',
      dependencies: { constructor: '1' },
      renovate: { labels: ['deps'], extends: ['group:vitePlus'] },
    });
  });

  const presets = ['config:recommended', 'group:vitePlus'];
  it.each([
    ['renovate.json', { extends: 'config:recommended' }, { extends: presets }],
    [
      'package.json',
      { name: 'demo', renovate: 'config:recommended' },
      { name: 'demo', renovate: { extends: presets } },
    ],
    [
      'package.json',
      { renovate: { extends: 'config:recommended', enabled: true } },
      { renovate: { extends: presets, enabled: true } },
    ],
  ] as const)('expands a single preset in %s: %j', async (name, original, expected) => {
    const file = await config(name, JSON.stringify(original));
    expect(await findRenovateConfigToUpdate(root)).toBe(file);
    expect(await addVitePlusRenovatePreset(file)).toBe(true);
    const updated = await fs.readFile(file, 'utf8');
    expect(JSON.parse(updated)).toEqual(expected);
    await expectSkipped(file, updated);
  });

  it.each([
    ['renovate.jsonc', '"extends"', String.raw`"config:recomm\u0065nded"`, '"group:vitePlus"'],
    ['renovate.json5', 'extends', String.raw`'config:recomm\x65nded'`, "'group:vitePlus'"],
  ])(
    'preserves the shorthand literal and surrounding comments in %s',
    async (name, key, literal, group) => {
      const source = `{\r\n  ${key}: /* before */ ${literal} /* after */,\r\n}\r\n`;
      const file = await config(name, source);
      expect(await findRenovateConfigToUpdate(root)).toBe(file);
      expect(await addVitePlusRenovatePreset(file)).toBe(true);
      const updated = await fs.readFile(file, 'utf8');
      expect(updated).toBe(source.replace(literal, `[${literal}, ${group}]`));
      await expectSkipped(file, updated);
    },
  );

  it.each([
    ['renovate.json', '{"extends":"group:vitePlus"}'],
    ['renovate.json5', String.raw`{extends:'group:vite\x50lus'}`],
    ['package.json', '{"renovate":"group:vitePlus"}'],
    ['package.json', '{"renovate":{"extends":"group:vitePlus"}}'],
  ])('skips an already enabled shorthand in %s: %s', async (name, source) => {
    expect.hasAssertions();
    await expectSkipped(await config(name, source), source);
  });

  it('ignores an empty Renovate shorthand in package.json', async () => {
    expect.hasAssertions();
    const source = '{"renovate":""}';
    await expectSkipped(await config('package.json', source), source);
  });

  it.each([
    '{',
    '[]',
    '{ "extends": null }',
    '{ "extends": {} }',
    '{ "extends": ["valid", 42] }',
    '{ "extends": ["first"], "extends": ["second"] }',
    '{ "extends": ["group:vitePlus"] }',
  ])(
    'skips an ineligible active config without touching a lower-priority config: %s',
    async (source) => {
      expect.hasAssertions();
      const file = await config('renovate.json', source);
      await config('.github/renovate.json', '{}');
      await expectSkipped(file, source);
    },
  );

  it('ignores projects without Renovate and does not recreate a config removed after detection', async () => {
    await config('package.json', '{"name":"demo"}');
    expect(await findRenovateConfigToUpdate(root)).toBeUndefined();
    const file = await config('.renovaterc', '{}');
    expect(await findRenovateConfigToUpdate(root)).toBe(file);
    await fs.unlink(file);
    expect(await addVitePlusRenovatePreset(file)).toBe(false);
    expect(await fs.readdir(root)).toEqual(['package.json']);
  });
});
