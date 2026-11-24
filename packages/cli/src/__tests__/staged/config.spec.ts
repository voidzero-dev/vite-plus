import { assertType, describe, it } from 'vitest';

import type { StagedConfig } from '../../staged-config.ts';

describe('StagedConfig type', () => {
  it('accepts string commands', () => {
    assertType<StagedConfig>({
      '*.ts': 'eslint --fix',
    });
  });

  it('accepts arrays of string commands', () => {
    assertType<StagedConfig>({
      '*.ts': ['eslint --fix', 'prettier --write'],
    });
  });

  it('accepts sync generate task functions', () => {
    assertType<StagedConfig>({
      '*.ts': (files: ReadonlyArray<string>) => `eslint ${files.join(' ')}`,
    });
  });

  it('accepts async generate task functions', () => {
    assertType<StagedConfig>({
      '*.ts': async (files: ReadonlyArray<string>) => `eslint ${files.join(' ')}`,
    });
  });

  it('accepts mixed arrays of strings and functions', () => {
    assertType<StagedConfig>({
      '*': [
        () => 'pnpm install --ignore-scripts',
        () => 'pnpm test',
        'oxlint --deny-warnings --fix',
        'prettier --ignore-unknown --write',
      ],
    });
  });

  it('accepts task function objects', () => {
    assertType<StagedConfig>({
      '*.ts': {
        title: 'Run eslint',
        task: (files: ReadonlyArray<string>) => {
          assertType<ReadonlyArray<string>>(files);
        },
      },
    });
  });

  it('accepts a top-level generate task function', () => {
    assertType<StagedConfig>((files: ReadonlyArray<string>) => [`eslint ${files.join(' ')}`]);
  });
});
