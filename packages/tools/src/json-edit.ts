import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

export function jsonEdit() {
  const { positionals } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
  });

  const filename = positionals[0];
  const script = positionals[1];

  if (!filename || !script) {
    console.error('Usage: tool json-edit <filename> <script>');
    console.error('Example: tool json-edit package.json \'_.version = "1.2.3"\'');
    process.exit(1);
  }

  const json = JSON.parse(readFileSync(filename, 'utf-8'));
  const func = new Function('_', script + '; return _;');
  const result = func(json);

  writeFileSync(filename, JSON.stringify(result, null, 2) + '\n', 'utf-8');
}
