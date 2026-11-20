import { spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { stripVTControlCharacters } from 'node:util';

const args = ['create', ...process.argv.slice(2)];
// Windows needs cmd to launch the local CLI's .cmd shim. All arguments are fixture literals.
const result =
  process.platform === 'win32'
    ? spawnSync('cmd.exe', ['/d', '/s', '/c', `vp ${args.join(' ')}`], { encoding: 'utf8' })
    : spawnSync('vp', args, { encoding: 'utf8' });
const output = stripVTControlCharacters(`${result.stdout ?? ''}${result.stderr ?? ''}`);
process.stdout.write(output);

// Keep the summary's exact package-manager version visible despite banner version redaction.
const summary = output.match(/• Node \S+\s+(\S+) (\S+)/);
writeFileSync(
  'create-summary.json',
  JSON.stringify({ packageManager: summary?.[1] ?? null, version: summary?.[2] ?? null }, null, 2) + '\n',
);
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
