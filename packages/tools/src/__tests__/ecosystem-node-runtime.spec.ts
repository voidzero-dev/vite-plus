import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { expect, test } from 'vitest';

import { patchDecodersNodeRuntime } from '../../../../ecosystem-ci/patch-node-runtime.ts';
import { planVitestV5Migration } from '../../../cli/src/migration/migrator.ts';
import { PackageManager } from '../../../cli/src/types/index.ts';

const workflow = `jobs:
  build:
    strategy:
      matrix:
        node-version: [
            '20.x', # EoL by 2026-04-30
            '22.x', # EoL by 2027-04-30
            '24.x', # EoL by 2028-04-30
            'latest',
          ]
    steps:
      - uses: actions/setup-node@v5
        with:
          node-version: \${{ matrix.node-version }}
`;

test.each(['\n', '\r\n'])('updates only the unsupported decoders matrix entry (%j)', (newline) => {
  const source = workflow.replaceAll('\n', newline);
  expect(patchDecodersNodeRuntime(source)).toBe(
    source.replace("'20.x', # EoL by 2026-04-30", "'26.x',"),
  );
});

test.each(['', workflow.replace("'20.x'", "'26.x'"), workflow + workflow])(
  'requires review when the upstream matrix changes (%#)',
  (source) => {
    expect(() => patchDecodersNodeRuntime(source)).toThrow('review the temporary Node 20');
  },
);

test('unblocks migration without changing the library engine contract', () => {
  const rootDir = mkdtempSync(join(tmpdir(), 'vp-ecosystem-node-'));
  try {
    const manifest = JSON.stringify({
      engines: { node: '>=18' },
      devDependencies: { vitest: '^4.1.4' },
    });
    writeFileSync(join(rootDir, 'package.json'), manifest);
    mkdirSync(join(rootDir, '.github/workflows'), { recursive: true });
    const workflowPath = join(rootDir, '.github/workflows/test.yml');
    writeFileSync(workflowPath, workflow);
    const blocks = () =>
      planVitestV5Migration({ rootDir, packageManager: PackageManager.npm }).findings.filter(
        ({ severity }) => severity === 'block',
      );
    expect(blocks()).toEqual([expect.objectContaining({ code: 'node-runtime' })]);
    writeFileSync(workflowPath, patchDecodersNodeRuntime(workflow));
    expect(blocks()).toEqual([]);
    expect(readFileSync(join(rootDir, 'package.json'), 'utf8')).toBe(manifest);
  } finally {
    rmSync(rootDir, { recursive: true, force: true });
  }
});
