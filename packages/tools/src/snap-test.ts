#!/usr/bin/env node

import cp from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const cpExec = promisify(cp.exec);
const exec = async (command: string, options: cp.ExecOptionsWithStringEncoding) =>
  cpExec(
    command,
    process.platform === 'win32' ? { ...options, shell: 'pwsh.exe' } : options,
  );

import { replaceUnstableOutput } from './utils.ts';

// Create a unique temporary directory for testing
const tempTmpDir = `${tmpdir()}/vite-plus-test-${randomUUID()}`;
fs.mkdirSync(tempTmpDir, { recursive: true });

// Clean up the temporary directory on exit
process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

const casesDir = path.resolve('snap-tests');

const filter = process.argv[2] ?? ''; // Optional filter to run specific test cases

// const tasks: Promise<void>[] = [];
for (const caseName of fs.readdirSync(casesDir)) {
  if (caseName.startsWith('.')) continue; // Skip hidden files like .DS_Store
  if (caseName.includes(filter)) {
    // FIXME: parallel run will cause [Error: Broken pipe (os error 32)] { code: 'GenericFailure' }
    // tasks.push(runTestCase(caseName));
    await runTestCase(caseName);
  }
}

// await Promise.all(tasks);

interface Steps {
  env: Record<string, string>;
  commands: string[];
}

async function runTestCase(name: string) {
  console.log('%s started', name);
  const caseTmpDir = `${tempTmpDir}/${name}`;
  await fsPromises.cp(`${casesDir}/${name}`, caseTmpDir, { recursive: true, errorOnExist: true });

  const steps: Steps = JSON.parse(await fsPromises.readFile(`${caseTmpDir}/steps.json`, 'utf-8'));

  const env = {
    ...process.env,
    // Indicate CLI is running in test mode, so that it prints more detailed outputs.
    VITE_PLUS_CLI_TEST: '1',
    NO_COLOR: 'true',
    // set CI=true make sure snap-tests are stable on GitHub Actions
    CI: 'true',

    // A test case can override/unset environment variables above.
    // For example, VITE_PLUS_CLI_TEST/CI can be unset to test the real-world outputs.
    ...steps.env,
  };

  // Sometimes on Windows, the PATH variable is named 'Path'
  if ('Path' in env && !('PATH' in env)) {
    env['PATH'] = env['Path'];
    delete env['Path'];
  }
  env['PATH'] = [
    // Extend PATH to include the package's bin directory
    path.resolve('bin'),
    ...env['PATH']!.split(path.delimiter),
  ].join(path.delimiter);

  const newSnap: string[] = [];

  for (const command of steps.commands) {
    try {
      const { stdout, stderr } = await exec(command, { env, cwd: caseTmpDir, encoding: 'utf-8' });
      newSnap.push(`> ${command}`);
      if (stdout) {
        newSnap.push(replaceUnstableOutput(stdout, caseTmpDir));
      }
      if (stderr) {
        newSnap.push(replaceUnstableOutput(stderr, caseTmpDir));
      }
    } catch (error) {
      // add error exit code to the command
      newSnap.push(`[${error.code}]> ${command}`);
      if (error.stdout) {
        newSnap.push(replaceUnstableOutput(error.stdout, caseTmpDir));
      }
      if (error.stderr) {
        newSnap.push(replaceUnstableOutput(error.stderr, caseTmpDir));
      }
    }
  }
  const newSnapContent = newSnap.join('\n');

  await fsPromises.writeFile(`${casesDir}/${name}/snap.txt`, newSnapContent);
  console.log('%s finished', name);
}
