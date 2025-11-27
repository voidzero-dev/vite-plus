import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { migratePackageJson } from '../migration.ts';

describe('migratePackageJson', () => {
  it('should migrate package.json scripts', async () => {
    const tempDir = await fs.mkdtemp(path.join(tmpdir(), 'vite-plus-test-'));
    await fs.writeFile(
      path.join(tempDir, 'package.json'),
      JSON.stringify(
        {
          scripts: {
            test: 'vitest',
            test_run: 'vitest run && vitest --ui',
            lint: 'oxlint',
            lint_config: 'oxlint --config .oxlint.json',
            lint_type_aware: 'oxlint --type-aware',
            fmt: 'oxfmt',
            fmt_config: 'oxfmt --config .oxfmt.json',
            lib: 'tsdown',
            lib_watch: 'tsdown --watch',
            preview: 'vite preview',
            optimize: 'vite optimize',
            build:
              'pnpm install &&vite build -r && vite run build --watch && tsdown && tsc || exit 1',
            dev: 'vite',
            dev_help: 'vite --help && vite -h',
            dev_port: 'vite --port 3000',
            dev_host: 'vite --host 0.0.0.0',
            dev_open: 'vite --open',
            dev_verbose: 'vite --verbose',
            dev_debug: 'vite --debug',
            dev_trace: 'vite --trace',
            dev_profile: 'vite --profile',
            dev_stats: 'vite --stats',
            dev_analyze: 'vite --analyze',
            ready: 'oxlint --fix --type-aware && vitest run && tsdown && oxfmt --fix',
            ready_new:
              'vite install && vite fmt && vite lint --type-aware && vite test -r && vite build -r',
          },
        },
        null,
        2,
      ),
    );
    const updated = await migratePackageJson(path.join(tempDir, 'package.json'));
    const scripts = JSON.parse(
      await fs.readFile(path.join(tempDir, 'package.json'), 'utf-8'),
    ).scripts;
    await fs.rm(tempDir, { recursive: true });
    expect(updated).toBe(true);
    expect(scripts).toMatchSnapshot();
  });
});
