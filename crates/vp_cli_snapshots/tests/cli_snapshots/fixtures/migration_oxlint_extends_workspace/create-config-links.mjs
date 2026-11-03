import { symlinkSync } from 'node:fs';

if (process.argv.includes('--owner-alias')) {
  symlinkSync('../../.oxlintrc.json', 'packages/a/alias.json', 'file');
  process.exit(0);
}

for (const name of ['a', 'b']) {
  symlinkSync('../../shared/lint.json', `packages/${name}/.oxlintrc.json`, 'file');
  if (process.argv.includes('--with-base')) {
    symlinkSync('../../shared/base.json', `packages/${name}/base.json`, 'file');
  }
}
