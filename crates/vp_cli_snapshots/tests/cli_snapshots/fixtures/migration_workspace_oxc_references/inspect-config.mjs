import { loadConfigFromFile } from 'vite-plus';

const loaded = await loadConfigFromFile(
  { command: 'build', mode: 'test' },
  './vite.config.ts',
  process.cwd(),
  'silent',
  undefined,
  'native',
);
console.log(loaded.config.define.FORMAT_STYLE);
