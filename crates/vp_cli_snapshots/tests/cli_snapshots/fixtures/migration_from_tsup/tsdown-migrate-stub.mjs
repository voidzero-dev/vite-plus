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

const packageJson = JSON.parse(fs.readFileSync('package.json', 'utf8'));
packageJson.scripts.build = packageJson.scripts.build.replace(/\btsup(?:-node)?\b/g, 'tsdown');
packageJson.devDependencies.tsdown = '0.22.14';
delete packageJson.devDependencies.tsup;
fs.writeFileSync('package.json', `${JSON.stringify(packageJson, null, 2)}\n`);

fs.writeFileSync(
  'tsdown.config.ts',
  `import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  splitting: false,
  target: false,
});
`,
);
fs.unlinkSync('tsup.config.ts');

console.error(
  '\n WARN  The splitting option is currently unsupported in tsdown. Code splitting is always enabled and cannot be disabled.\n',
);
