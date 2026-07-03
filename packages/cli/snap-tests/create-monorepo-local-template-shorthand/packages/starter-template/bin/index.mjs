#!/usr/bin/env node
// Minimal local template generator. Writes a package that declares its `fmt`
// and `lint` config via shorthand properties, plus standalone Oxlint/Oxfmt
// config files. `vp create` must merge-skip the standalone files instead of
// injecting duplicate inline `fmt:`/`lint:` blocks into vite.config.ts (#1836).
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const dirFlag = args.indexOf('--directory');
const dir = dirFlag !== -1 && args[dirFlag + 1] ? args[dirFlag + 1] : 'starter-app';

mkdirSync(dir, { recursive: true });

const write = (name, content) => writeFileSync(path.join(dir, name), content);

write(
  'package.json',
  `${JSON.stringify({ name: path.basename(dir), version: '0.0.0', private: true }, null, 2)}\n`,
);

write(
  'vite.config.ts',
  `import { defineConfig } from 'vite-plus';

import { fmt } from './tooling/format';
import { lint } from './tooling/lint';

export default defineConfig(({ mode }) => {
  return {
    server: { port: 3000 },
    fmt,
    lint,
  };
});
`,
);

write('.oxlintrc.json', `${JSON.stringify({ rules: {} }, null, 2)}\n`);
write('.oxfmtrc.json', `${JSON.stringify({}, null, 2)}\n`);

mkdirSync(path.join(dir, 'tooling'), { recursive: true });
writeFileSync(
  path.join(dir, 'tooling', 'format.ts'),
  'export const fmt = { ignorePatterns: [] };\n',
);
writeFileSync(path.join(dir, 'tooling', 'lint.ts'), 'export const lint = { rules: {} };\n');

console.log(`cloned starter-template to ${dir}`);
