import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

export function replaceFileContent() {
  const { positionals } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
  });

  const filename = positionals[0];
  const searchValue = positionals[1];
  const newValue = positionals[2];

  if (!filename || !searchValue || !newValue) {
    console.error('Usage: tool replace-file-content <filename> <searchValue> <newValue>');
    console.error(
      'Example: tool replace-file-content example.toml \'version = "0.0.0"\' \'version = "0.0.1"\'',
    );
    process.exit(1);
  }

  const filepath = path.resolve(filename);
  const content = readFileSync(filepath, 'utf-8');
  const newContent = content.replace(searchValue, newValue);
  writeFileSync(filepath, newContent, 'utf-8');
}
