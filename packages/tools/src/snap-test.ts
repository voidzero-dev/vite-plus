import { randomUUID } from 'node:crypto';
import fs, { readFileSync } from 'node:fs';
import fsPromises from 'node:fs/promises';
import { open } from 'node:fs/promises';
import { cpus, homedir, tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout } from 'node:timers/promises';
import { debuglog, parseArgs } from 'node:util';

import { npath } from '@yarnpkg/fslib';
import { execute } from '@yarnpkg/shell';

import { isPassThroughEnv, replaceUnstableOutput } from './utils';

const debug = debuglog('vite-plus/snap-test');

// Remove comments (starting with ' #') from command strings
// `@yarnpkg/shell` doesn't parse comments.
// This doesn't handle all edge cases (such as ' #' in quoted strings), but is good enough for our test cases.
function stripComments(command: string): string {
  if (command.trim().startsWith('#')) {
    return '';
  }
  const commentStart = command.indexOf(' #');
  return commentStart === -1 ? command : command.slice(0, commentStart);
}

/**
 * Run tasks with limited concurrency based on CPU count.
 * @param tasks Array of task functions to execute
 * @param maxConcurrency Maximum number of concurrent tasks (defaults to CPU count)
 */
async function runWithConcurrencyLimit(
  tasks: (() => Promise<void>)[],
  maxConcurrency = cpus().length,
): Promise<void> {
  const executing: Promise<void>[] = [];
  const errors: Error[] = [];

  for (const task of tasks) {
    const promise = task()
      .catch((error) => {
        errors.push(error);
        console.error('Task failed:', error);
      })
      .finally(() => {
        executing.splice(executing.indexOf(promise), 1);
      });

    executing.push(promise);

    if (executing.length >= maxConcurrency) {
      await Promise.race(executing);
    }
  }

  await Promise.all(executing);

  if (errors.length > 0) {
    throw new Error(`${errors.length} test case(s) failed. First error: ${errors[0].message}`);
  }
}

export async function snapTest() {
  const { positionals } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
  });

  const filter = positionals[0] ?? ''; // Optional filter to run specific test cases

  // Create a unique temporary directory for testing
  // On macOS, `tmpdir()` is a symlink. Resolve it so that we can replace the resolved cwd in outputs.
  const systemTmpDir = fs.realpathSync(tmpdir());
  const tempTmpDir = `${systemTmpDir}/vite-plus-test-${randomUUID()}`;
  fs.mkdirSync(tempTmpDir, { recursive: true });

  // Clean up stale .node-version and package.json in the system temp directory.
  // vite-plus walks up the directory tree to resolve Node.js versions, so leftover
  // files from previous runs can cause tests to pick up unexpected version configs.
  for (const staleFile of ['.node-version', 'package.json']) {
    const stalePath = path.join(systemTmpDir, staleFile);
    if (fs.existsSync(stalePath)) {
      fs.rmSync(stalePath);
    }
  }

  const vitePlusHome = path.join(homedir(), '.vite-plus');

  // Remove .previous-version so command-self-update-rollback snap test is stable
  const previousVersionPath = path.join(vitePlusHome, '.previous-version');
  if (fs.existsSync(previousVersionPath)) {
    fs.rmSync(previousVersionPath);
  }

  // Ensure shim mode is "managed" so snap tests use vite-plus managed Node.js
  // instead of the system Node.js (equivalent to running `vp env on`).
  const configPath = path.join(vitePlusHome, 'config.json');
  if (fs.existsSync(configPath)) {
    const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    if (config.shimMode && config.shimMode !== 'managed') {
      delete config.shimMode;
      fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');
    }
  }

  // Make dependencies available in the test cases
  fs.symlinkSync(
    path.resolve('node_modules'),
    path.join(tempTmpDir, 'node_modules'),
    process.platform === 'win32' ? 'junction' : 'dir',
  );

  // Clean up the temporary directory on exit
  process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

  const casesDir = path.resolve('snap-tests');

  const taskFunctions: (() => Promise<void>)[] = [];
  for (const caseName of fs.readdirSync(casesDir)) {
    if (caseName.startsWith('.')) continue; // Skip hidden files like .DS_Store
    if (caseName.includes(filter)) {
      taskFunctions.push(() => runTestCase(caseName, tempTmpDir, casesDir));
    }
  }

  if (taskFunctions.length > 0) {
    const cpuCount = cpus().length;
    console.log(
      'Running %d test cases with concurrency limit of %d (CPU count)',
      taskFunctions.length,
      cpuCount,
    );
    await runWithConcurrencyLimit(taskFunctions, cpuCount);
  }
  process.exit(0); // Ensure exit even if there are pending timed-out steps
}

interface Command {
  command: string;
  /**
   * If true, the stdout and stderr output of the command will be ignored.
   * This is useful for commands that stdout/stderr is unstable.
   */
  ignoreOutput?: boolean;
  /**
   * The timeout in milliseconds for the command.
   * If not specified, the default timeout is 50 seconds.
   */
  timeout?: number;
}

interface Steps {
  ignoredPlatforms?: string[];
  env: Record<string, string>;
  commands: (string | Command)[];
  /**
   * Commands to run after the test completes, regardless of success or failure.
   * Useful for cleanup tasks like killing background processes.
   * These commands are not included in the snap output.
   */
  after?: string[];
}

async function runTestCase(name: string, tempTmpDir: string, casesDir: string) {
  const steps: Steps = JSON.parse(
    await fsPromises.readFile(`${casesDir}/${name}/steps.json`, 'utf-8'),
  );
  if (steps.ignoredPlatforms !== undefined && steps.ignoredPlatforms.includes(process.platform)) {
    console.log('%s skipped on platform %s', name, process.platform);
    return;
  }

  console.log('%s started', name);
  const caseTmpDir = `${tempTmpDir}/${name}`;
  await fsPromises.cp(`${casesDir}/${name}`, caseTmpDir, {
    recursive: true,
    errorOnExist: true,
  });

  const passThroughEnvs = Object.fromEntries(
    Object.entries(process.env).filter(([key]) => isPassThroughEnv(key)),
  );
  const env: Record<string, string> = {
    ...passThroughEnvs,
    // Indicate CLI is running in test mode, so that it prints more detailed outputs.
    VITE_PLUS_CLI_TEST: '1',
    NO_COLOR: 'true',
    // set CI=true make sure snap-tests are stable on GitHub Actions
    CI: 'true',
    VITE_PLUS_HOME: path.join(homedir(), '.vite-plus'),

    // A test case can override/unset environment variables above.
    // For example, VITE_PLUS_CLI_TEST/CI can be unset to test the real-world outputs.
    ...steps.env,
  };

  // Unset VITE_PLUS_NODE_VERSION to prevent `vp env use` session overrides
  // from leaking into snap tests (it passes through via the VITE_* pattern).
  delete env['VITE_PLUS_NODE_VERSION'];

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

  const startTime = Date.now();
  const cwd = npath.toPortablePath(caseTmpDir);

  try {
    for (const command of steps.commands) {
      const cmd = typeof command === 'string' ? { command } : command;
      debug('running command: %o, cwd: %s, env: %o', cmd, caseTmpDir, env);

      // While `@yarnpkg/shell` supports capturing output via in-memory `Writable` streams,
      // it seems not to have stable ordering of stdout/stderr chunks.
      // To ensure stable ordering, we redirect outputs to a file instead.
      const outputStreamPath = path.join(caseTmpDir, 'output.log');
      const outputStream = await open(outputStreamPath, 'w');

      const exitCode = await Promise.race([
        execute(stripComments(cmd.command), [], {
          env,
          cwd,
          stdin: null,
          // Declared to be `Writable` but `FileHandle` works too.
          // @ts-expect-error
          stderr: outputStream,
          // @ts-expect-error
          stdout: outputStream,
          glob: {
            // Disable glob expansion. Pass args like '--filter=*' as-is.
            isGlobPattern: () => false,
            match: async () => [],
          },
        }),
        setTimeout(cmd.timeout ?? 50 * 1000),
      ]);

      await outputStream.close();

      let output = readFileSync(outputStreamPath, 'utf-8');

      let commandLine = `> ${cmd.command}`;
      if (exitCode !== 0) {
        commandLine = (exitCode === undefined ? '[timeout]' : `[${exitCode}]`) + commandLine;
      } else {
        // only allow ignore output if the command is successful
        if (cmd.ignoreOutput) {
          output = '';
        }
      }
      newSnap.push(commandLine);
      if (output.length > 0) {
        newSnap.push(replaceUnstableOutput(output, caseTmpDir));
      }
      if (exitCode === undefined) {
        break; // Stop executing further commands on timeout
      }
    }
  } finally {
    // Run after commands for cleanup, regardless of success or failure
    if (steps.after) {
      for (const afterCmd of steps.after) {
        debug('running after command: %s, cwd: %s', afterCmd, caseTmpDir);
        try {
          await execute(stripComments(afterCmd), [], {
            env,
            cwd,
            stdin: null,
          });
        } catch (error) {
          debug('after command failed: %s, error: %o', afterCmd, error);
        }
      }
    }
  }

  const newSnapContent = newSnap.join('\n');

  await fsPromises.writeFile(`${casesDir}/${name}/snap.txt`, newSnapContent);
  console.log('%s finished in %dms', name, Date.now() - startTime);
}
