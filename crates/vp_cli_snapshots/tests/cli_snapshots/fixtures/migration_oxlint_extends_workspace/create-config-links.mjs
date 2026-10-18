import { symlinkSync } from 'node:fs';

for (const name of ['a', 'b']) {
  symlinkSync('../../shared/lint.json', `packages/${name}/.oxlintrc.json`, 'file');
}
