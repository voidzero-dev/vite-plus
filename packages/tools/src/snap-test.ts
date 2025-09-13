#!/usr/bin/env node

import cp from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const exec = promisify(cp.exec);

// Create a unique temporary directory for testing
const tempTmpDir = `${tmpdir()}/vite-plus-test-${randomUUID()}`;
fs.mkdirSync(tempTmpDir, { recursive: true });

// Clean up the temporary directory on exit
process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

const casesDir = path.resolve('snap-tests');

const filter = process.argv[2] ?? ''; // Optional filter to run specific test cases

const tasks: Promise<void>[] = [];
for (const caseName of fs.readdirSync(casesDir)) {
  if (caseName.startsWith('.')) continue; // Skip hidden files like .DS_Store
  if (caseName.includes(filter)) {
    tasks.push(runTestCase(caseName));
  }
}

await Promise.all(tasks);

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
    ...steps.env,
    // Indicate CLI is running in test mode
    VITE_PLUS_CLI_TEST: '1',
  };
  env['PATH'] = [
    ...env['PATH']!.split(path.delimiter),
    // Extend PATH to include the package's bin directory
    path.resolve('bin')
  ].join(path.delimiter);

  const newSnap: string[] = [];

  for (const command of steps.commands) {
    try {
      const { stdout, stderr } = await exec(command, { env, cwd: caseTmpDir, encoding: 'utf-8' });
      newSnap.push(`> ${command}`);
      if (stdout) {
        newSnap.push(replaceUnstableOutput(stdout));
      }
      if (stderr) {
        newSnap.push(replaceUnstableOutput(stderr));
      }
    } catch (error) {
      // add error exit code to the command
      newSnap.push(`[${error.code}]> ${command}`);
      if (error.stdout) {
        newSnap.push(replaceUnstableOutput(error.stdout));
      }
      if (error.stderr) {
        newSnap.push(replaceUnstableOutput(error.stderr));
      }
    }
  }
  const newSnapContent = newSnap.join('\n');

  await fsPromises.writeFile(`${casesDir}/${name}/snap.txt`, newSnapContent);
  console.log('%s finished', name);
}

function replaceUnstableOutput(output: string) {
  return output.replace(/\d+(?:\.\d+)?s|\d+ms/, '<variable>ms')
    .replace(/with \d+ rules using \d+ threads/, 'with <variable> rules using <variable> threads');
}
