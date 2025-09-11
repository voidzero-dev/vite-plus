import assert from 'node:assert';
import cp from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

// Create a unique temporary directory for testing
const tempTmpDir = `${tmpdir()}/vite-plus-test-${randomUUID()}`;
fs.mkdirSync(tempTmpDir, { recursive: true });

// Clean up the temporary directory on exit
process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

const casesDir = import.meta.dirname + '/cases';

for (const caseName of fs.readdirSync(casesDir)) {
  if (caseName.startsWith('.')) continue; // Skip hidden files like .DS_Store
  runTestCase(caseName);
}

interface Steps {
  env: Record<string, string>;
  commands: string[];
}

function runTestCase(name: string) {
  const caseTmpDir = `${tempTmpDir}/${name}`;
  fs.cpSync(`${casesDir}/${name}`, caseTmpDir, { recursive: true, errorOnExist: true });

  const steps: Steps = JSON.parse(fs.readFileSync(`${caseTmpDir}/steps.json`, 'utf-8'));

  const env = {
    ...process.env,
    ...steps.env,
    // Indicate CLI is running in test mode
    VITE_PLUS_CLI_TEST: '1',
  };
  env['PATH'] = [
    ...env['PATH']!.split(path.delimiter),
    // Extend PATH to include the CLI's bin directory
    path.dirname(import.meta.dirname) + '/bin',
    // Also include node_modules/.bin for local dev dependencies
    path.dirname(import.meta.dirname) + '/node_modules/.bin',
  ].join(path.delimiter);

  const newSnap: string[] = [];

  for (const command of steps.commands) {
    newSnap.push(`> ${command}`);
    const output = cp.execSync(command, { env, cwd: caseTmpDir, encoding: 'utf8' });
    newSnap.push(output);
  }
  const newSnapContent = newSnap.join('\n');

  fs.writeFileSync(`${casesDir}/${name}/snap.txt`, newSnapContent);
}
