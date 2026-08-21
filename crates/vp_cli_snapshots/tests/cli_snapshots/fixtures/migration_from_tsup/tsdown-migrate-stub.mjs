#!/usr/bin/env node
import fs from 'node:fs';

const args = process.argv.slice(2);
const validArgs =
  args[0] === 'dlx' &&
  args[1]?.startsWith('tsdown-migrate@') &&
  args[2] === '--yes' &&
  args[3] === '--package-manager' &&
  args[4] === 'pnpm' &&
  args[5] === '--no-install';

if (!validArgs || process.env.TSDOWN_MIGRATE_STUB_FAIL === '1') {
  process.exit(1);
}

fs.writeFileSync(
  'tsdown.config.ts',
  `import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  target: false,
});
`,
);
