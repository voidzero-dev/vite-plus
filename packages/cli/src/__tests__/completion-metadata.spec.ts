import { mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vite-plus/test';
import { createCLI } from 'vitest/node';

import {
  extractCompletionMetadata,
  writeTestCompletionMetadata,
} from '../../build/completion-metadata.ts';
import { VITEST_VERSION } from '../utils/constants.ts';

const temporary: string[] = [];
afterEach(async () => {
  await Promise.all(temporary.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});

// Use the public CAC builder with controlled declarations. None of the flags
// below asserts that a particular Vitest release must ship that option.
function fixture() {
  const cli = createCLI();
  cli.commands.splice(0);
  cli.globalCommand.options.splice(0);
  return cli;
}

function extract(cli = fixture()) {
  return extractCompletionMetadata(cli, '1.3.0', VITEST_VERSION);
}

describe('Vitest completion metadata adapter (synthetic declarations)', () => {
  it('preserves boolean, required and optional declarations and short aliases', () => {
    const cli = fixture();
    cli.option('-b, --contractboolean', 'Boolean');
    cli.option('-r, --contractrequired <value>', 'Required');
    cli.option('--contractoptional [value]', 'Optional');
    expect(extract(cli).globalOptions).toEqual([
      {
        flags: ['-b', '--contractboolean'],
        description: 'Boolean',
        valueMode: 'none',
      },
      {
        flags: ['-r', '--contractrequired'],
        description: 'Required',
        valueMode: 'required',
      },
      {
        flags: ['--contractoptional'],
        description: 'Optional',
        valueMode: 'optional',
      },
    ]);
  });

  it('preserves negative spelling, dot options and nested placeholders', () => {
    const cli = fixture();
    cli.option('--no-contractcolor', 'Negative');
    cli.option('--contractinspect [[host:]port]', 'Nested placeholder');
    cli.option('--contractcoverage.reporter <name>', 'Dot option');
    expect(extract(cli).globalOptions).toEqual([
      {
        flags: ['--no-contractcolor'],
        description: 'Negative',
        valueMode: 'none',
      },
      {
        flags: ['--contractinspect'],
        description: 'Nested placeholder',
        valueMode: 'optional',
      },
      {
        flags: ['--contractcoverage.reporter'],
        description: 'Dot option',
        valueMode: 'required',
      },
    ]);
  });

  it('keeps local descriptions and additions without inventing inherited options', () => {
    const cli = fixture();
    cli.option('--contractshared <value>', 'Global description');
    cli
      .command('contract')
      .option('--contractshared <value>', 'Local description')
      .option('--contractlocal', 'Local only');
    const data = extract(cli);
    expect(data.globalOptions).toHaveLength(1);
    expect(data.commands[0].options.map((option) => option.description)).toEqual([
      'Local description',
      'Local only',
    ]);
  });

  it('accepts empty inventories and preserves future command definitions', () => {
    expect(extract()).toMatchObject({ globalOptions: [], commands: [] });
    const cli = fixture();
    cli
      .command('future-command [...filters]', 'Future description')
      .alias('future-alias')
      .option('--future-flag', 'Future option');
    expect(extract(cli).commands).toEqual([
      {
        name: 'future-command',
        options: [
          {
            flags: ['--future-flag'],
            description: 'Future option',
            valueMode: 'none',
          },
        ],
      },
    ]);
  });

  it('rejects duplicate declarations instead of silently replacing them', () => {
    const cli = fixture();
    cli.option('--contractduplicate', 'Duplicate');
    cli.globalCommand.options.push(cli.globalCommand.options[0]);
    expect(() => extract(cli)).toThrow('Duplicate Vitest option flag');
  });

  it('rejects ambiguous arity and incompatible inherited declarations', () => {
    const cli = fixture();
    cli.option('--contractvalue', 'Global boolean');
    cli.command('contract').option('--contractvalue <value>', 'Local required value');
    expect(() => extract(cli)).toThrow('Incompatible inherited Vitest option');
    const ambiguous = fixture();
    ambiguous.option('--contractambiguous', 'Ambiguous');
    ambiguous.globalCommand.options[0].required = true;
    expect(() => extract(ambiguous)).toThrow('Unsupported Vitest option arity');
  });

  it('rejects unsupported spelling instead of emitting unusable metadata', () => {
    const cli = fixture();
    cli.option('--contractflag', 'Invalid spelling');
    cli.globalCommand.options[0].rawName = '--bad/flag';
    expect(() => extract(cli)).toThrow('Unsupported Vitest option declaration');
  });

  it.each(['-ab, --contractflag', '--' + 'x'.repeat(257)])(
    'rejects declarations outside the Rust flag schema: %s',
    (rawName) => {
      const cli = fixture();
      cli.option('--contractflag', 'Flag');
      cli.globalCommand.options[0].rawName = rawName;
      expect(() => extract(cli)).toThrow('Unsupported Vitest option declaration');
    },
  );

  it('rejects inherited alias collisions across different primary options', () => {
    const cli = fixture();
    cli.option('-s, --contractshared', 'Global');
    cli.command('contract').option('-s, --contractlocal', 'Local');
    expect(() => extract(cli)).toThrow('Conflicting inherited Vitest option flag');
  });

  it('enforces command uniqueness and byte/count schema boundaries', () => {
    const invalidName = fixture();
    invalidName.command('contract');
    invalidName.commands[0].name = 'bad/name';
    expect(() => extract(invalidName)).toThrow('Unsupported Vitest command declaration');
    const duplicate = fixture();
    duplicate.command('contract');
    duplicate.command('contract');
    expect(() => extract(duplicate)).toThrow('Unsupported Vitest command declaration');
    const oversized = fixture();
    oversized.option('--contractflag', '文'.repeat(5462));
    expect(() => extract(oversized)).toThrow('Unsupported Vitest option declaration');
    const excessive = fixture();
    for (let i = 0; i < 65; i++) {
      excessive.command(`command-${i}`);
    }
    expect(() => extract(excessive)).toThrow('exceeds schema limits');
    const tooLarge = fixture();
    for (let i = 0; i < 70; i++) {
      tooLarge.option(`--contract-${i}`, 'x'.repeat(16384));
    }
    expect(() => extract(tooLarge)).toThrow('exceeds schema limits');
  });

  it('writes a package-relative artifact and rejects a mismatched runner', async () => {
    const directory = await mkdtemp(path.join(os.tmpdir(), 'vp-completion-'));
    temporary.push(directory);
    await writeFile(path.join(directory, 'package.json'), '{"name":"vite-plus","version":"1.2.0"}');
    const require = createRequire(import.meta.url);
    // Resolve through the same CLI package used by these tests, including pnpm layouts.
    const dependencyDirectory = path.resolve(
      path.dirname(require.resolve('vitest/package.json')),
      '..',
    );
    await symlink(dependencyDirectory, path.join(directory, 'node_modules'), 'junction');
    const metadata = await writeTestCompletionMetadata(directory, VITEST_VERSION);
    expect(
      JSON.parse(await readFile(path.join(directory, 'dist/completion/test.json'), 'utf8')),
    ).toEqual(metadata);
    await expect(writeTestCompletionMetadata(directory, '0.0.0')).rejects.toThrow(
      'Expected vitest@',
    );
  });
});
