import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const cliBinPath = fileURLToPath(new URL('../../../dist/bin.js', import.meta.url));

describe('npm CLI sync-versions command', () => {
  it('runs the bundled planner instead of tree-shaking the dynamic import', () => {
    const before = '{"devDependencies":{"vite-plus":"0.0.0"}}\n';
    const request = JSON.stringify({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'package.json', kind: 'packageJson', contents: before }],
    });

    const stdout = execFileSync(process.execPath, [cliBinPath, 'sync-versions', '--json'], {
      input: request,
      encoding: 'utf8',
    });
    const plan = JSON.parse(stdout) as {
      tool: { name: string; version: string };
      replacements: Array<{ before: string; after: string }>;
    };

    expect(plan.tool.name).toBe('vite-plus');
    expect(plan.tool.version).toMatch(/^\d+\.\d+\.\d+/u);
    expect(plan.replacements).toEqual([
      {
        path: 'package.json',
        kind: 'packageJson',
        before,
        after: `{"devDependencies":{"vite-plus":"${plan.tool.version}"}}\n`,
      },
    ]);
  });
});
