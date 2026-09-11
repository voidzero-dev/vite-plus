import fs from 'node:fs/promises';

import { readBoundedUtf8 } from './input.ts';
import { runSyncVersionsProtocol } from './protocol.ts';

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function syncVersionsArgs(argv: readonly string[]): string[] {
  const args = argv.slice(2);
  return args[0] === 'sync-versions' ? args.slice(1) : args;
}

async function main(): Promise<void> {
  const args = syncVersionsArgs(process.argv);
  if (args.length !== 1 || args[0] !== '--json') {
    throw new Error('Usage: vp sync-versions --json');
  }

  if (process.stdin.isTTY) {
    throw new Error(
      'Expected a JSON request on stdin. Pipe the request to this command; it is intended for external automation.',
    );
  }

  const [requestJson, manifestJson] = await Promise.all([
    readBoundedUtf8(process.stdin),
    fs.readFile(new URL('../toolchain.json', import.meta.url), 'utf8'),
  ]);

  let manifest: unknown;
  try {
    manifest = JSON.parse(manifestJson);
  } catch {
    throw new Error('Invalid bundled toolchain manifest');
  }

  process.stdout.write(runSyncVersionsProtocol(requestJson, manifest));
}

try {
  await main();
} catch (error) {
  process.stderr.write(`vite-plus sync-versions: ${errorMessage(error)}\n`);
  process.exitCode = 1;
}
