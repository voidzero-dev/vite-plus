import { symlinkSync } from 'node:fs';

symlinkSync(
  '../../tooling/tsdown-library.ts',
  new URL('./packages/lib/tsdown.config.ts', import.meta.url),
  'file',
);
