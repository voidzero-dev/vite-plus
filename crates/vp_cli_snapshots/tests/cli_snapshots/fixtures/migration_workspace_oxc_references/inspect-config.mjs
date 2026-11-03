import { loadConfigFromFile } from 'vite-plus';

const loaded = await loadConfigFromFile(
  { command: 'build', mode: 'test' },
  process.argv[3] ?? './vite.config.ts',
  process.cwd(),
  'silent',
  undefined,
  process.argv[2] ?? 'native',
);
console.log(loaded.config.define.FORMAT_STYLE);
