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

import { isPassThroughEnv, replaceUnstableOutput } from './utils.js';

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
        // oxlint-disable-next-line typescript/no-floating-promises
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

function expandHome(p: string): string {
  return p.startsWith('~') ? path.join(homedir(), p.slice(1)) : p;
}

function parseShard(value: string): { index: number; total: number } {
  const match = value.match(/^(\d+)\/(\d+)$/);
  if (!match) {
    throw new Error(
      `Invalid --shard format: "${value}". Expected format: --shard=<index>/<total> (e.g., --shard=1/3)`,
    );
  }
  const index = Number(match[1]);
  const total = Number(match[2]);
  if (total < 1) {
    throw new Error(`Invalid --shard total: ${total}. Must be >= 1`);
  }
  if (index < 1 || index > total) {
    throw new Error(`Invalid --shard index: ${index}. Must be between 1 and ${total}`);
  }
  return { index, total };
}

function selectShard<T>(items: T[], index: number, total: number): T[] {
  const chunkSize = Math.ceil(items.length / total);
  const start = (index - 1) * chunkSize;
  return items.slice(start, start + chunkSize);
}

const NPM_GLOBAL_PREFIX_DIR = 'npm-global-lib-for-snap-tests';

export async function snapTest() {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
    options: {
      dir: { type: 'string' },
      'bin-dir': { type: 'string' },
      shard: { type: 'string' },
    },
  });

  const filter = positionals[0] ?? ''; // Optional filter to run specific test cases
  const shard = values.shard ? parseShard(values.shard) : undefined;

  // Create a unique temporary directory for testing
  // On macOS, `tmpdir()` is a symlink. Resolve it so that we can replace the resolved cwd in outputs.
  // Remove hyphens from UUID to avoid npm's @npmcli/redact treating the path as containing
  // secrets (it matches UUID patterns like `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
  const systemTmpDir = fs.realpathSync(tmpdir());
  const tempTmpDir = `${systemTmpDir}/vite-plus-test-${randomUUID().replaceAll('-', '')}`;
  fs.mkdirSync(tempTmpDir, { recursive: true });
  // Pre-create the npm global prefix directory so tests using npm global
  // operations (link, outdated -g, etc.) don't fail with ENOENT.
  fs.mkdirSync(path.join(tempTmpDir, NPM_GLOBAL_PREFIX_DIR, 'lib'), { recursive: true });

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

  // Remove .previous-version so command-upgrade-rollback snap test is stable
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

  // Make dependencies available in the test cases.
  // Create a real node_modules directory so we can add the CLI package itself
  // alongside the symlinked dependencies (needed for `vite-plus/*` imports in
  // vite.config.ts).
  const tempNodeModules = path.join(tempTmpDir, 'node_modules');
  fs.mkdirSync(tempNodeModules);
  const cliNodeModules = path.resolve('node_modules');
  for (const entry of fs.readdirSync(cliNodeModules)) {
    fs.symlinkSync(
      path.join(cliNodeModules, entry),
      path.join(tempNodeModules, entry),
      process.platform === 'win32' ? 'junction' : 'dir',
    );
  }
  // Add the CLI package itself so `vite-plus/*` subpath imports resolve
  fs.symlinkSync(
    path.resolve('.'),
    path.join(tempNodeModules, 'vite-plus'),
    process.platform === 'win32' ? 'junction' : 'dir',
  );

  // Clean up the temporary directory on exit
  process.on('exit', () => fs.rmSync(tempTmpDir, { recursive: true, force: true }));

  const casesDir = path.resolve(values.dir || 'snap-tests');

  // Collect valid test case names (sorted for deterministic sharding)
  const validCaseNames: string[] = [];
  const missingStepsJson: string[] = [];
  for (const caseName of fs.readdirSync(casesDir).toSorted()) {
    if (caseName.startsWith('.')) {
      continue;
    }
    const caseDir = path.join(casesDir, caseName);
    if (!fs.statSync(caseDir).isDirectory()) {
      continue;
    }
    const stepsPath = path.join(caseDir, 'steps.json');
    if (!fs.existsSync(stepsPath)) {
      missingStepsJson.push(caseName);
      continue;
    }
    if (caseName.includes(filter)) {
      validCaseNames.push(caseName);
    }
  }

  if (missingStepsJson.length > 0) {
    throw new Error(
      `${missingStepsJson.length} test case(s) missing steps.json: ${missingStepsJson.join(', ')}`,
    );
  }

  // Apply sharding to select a subset of test cases
  const selectedCases = shard
    ? selectShard(validCaseNames, shard.index, shard.total)
    : validCaseNames;

  const serialTasks: (() => Promise<void>)[] = [];
  const parallelTasks: (() => Promise<void>)[] = [];
  for (const caseName of selectedCases) {
    const stepsPath = path.join(casesDir, caseName, 'steps.json');
    const steps: Steps = JSON.parse(readFileSync(stepsPath, 'utf-8'));
    const task = () => runTestCase(caseName, tempTmpDir, casesDir, values['bin-dir']);
    if (steps.serial) {
      serialTasks.push(task);
    } else {
      parallelTasks.push(task);
    }
  }

  const totalCount = serialTasks.length + parallelTasks.length;
  if (totalCount > 0) {
    const cpuCount = cpus().length;
    const shardInfo = shard ? `, shard ${shard.index}/${shard.total}` : '';
    console.log(
      'Running %d test cases (%d serial + %d parallel, concurrency limit %d%s)',
      totalCount,
      serialTasks.length,
      parallelTasks.length,
      cpuCount,
      shardInfo,
    );
    await runWithConcurrencyLimit(serialTasks, 1);
    await runWithConcurrencyLimit(parallelTasks, cpuCount);
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

interface PlatformFilter {
  os: string;
  libc?: string;
}

interface Steps {
  ignoredPlatforms?: (string | PlatformFilter)[];
  env: Record<string, string>;
  commands: (string | Command)[];
  /**
   * Commands to run after the test completes, regardless of success or failure.
   * Useful for cleanup tasks like killing background processes.
   * These commands are not included in the snap output.
   */
  after?: string[];
  /**
   * If true, this test case will run serially before parallel tests.
   * Use for tests that modify global shared state (e.g., `vp env default`).
   */
  serial?: boolean;
}

let _isMusl: boolean | null = null;

function isMusl(): boolean {
  if (_isMusl === null) {
    if (process.platform !== 'linux') {
      _isMusl = false;
    } else if (typeof process.report?.getReport === 'function') {
      // Use Node.js process.report API to detect libc type:
      // - glibcVersionRuntime present → glibc
      // - shared objects contain "musl" → musl
      const report = process.report.getReport() as Record<string, any>;
      if (report.header?.glibcVersionRuntime) {
        _isMusl = false;
      } else if (Array.isArray(report.sharedObjects)) {
        _isMusl = report.sharedObjects.some(
          (f: string) => f.includes('libc.musl-') || f.includes('ld-musl-'),
        );
      } else {
        _isMusl = false;
      }
    } else {
      _isMusl = false;
    }
  }
  return _isMusl;
}

function shouldSkipPlatform(ignoredPlatforms: (string | PlatformFilter)[]): boolean {
  for (const filter of ignoredPlatforms) {
    if (typeof filter === 'string') {
      if (filter === process.platform) {
        return true;
      }
    } else {
      if (filter.os !== process.platform) {
        continue;
      }
      if (filter.libc === undefined) {
        return true;
      }
      if (filter.libc === 'musl' && isMusl()) {
        return true;
      }
      if (filter.libc === 'glibc' && !isMusl()) {
        return true;
      }
    }
  }
  return false;
}

async function runTestCase(name: string, tempTmpDir: string, casesDir: string, binDir?: string) {
  const steps: Steps = JSON.parse(
    await fsPromises.readFile(`${casesDir}/${name}/steps.json`, 'utf-8'),
  );
  if (steps.ignoredPlatforms !== undefined && shouldSkipPlatform(steps.ignoredPlatforms)) {
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
    // Also disables tips for stable snapshots.
    VP_CLI_TEST: '1',
    // Suppress Node.js runtime warnings (e.g. MODULE_TYPELESS_PACKAGE_JSON)
    // to keep snap outputs stable across Node.js versions.
    NODE_NO_WARNINGS: '1',
    NO_COLOR: 'true',
    // set CI=true make sure snap-tests are stable on GitHub Actions
    CI: 'true',
    VP_HOME: path.join(homedir(), '.vite-plus'),
    // Set git identity so `git commit` works on CI runners without global git config
    GIT_AUTHOR_NAME: 'Test',
    GIT_COMMITTER_NAME: 'Test',
    GIT_AUTHOR_EMAIL: 'vite-plus-test@test.com',
    GIT_COMMITTER_EMAIL: 'vite-plus-test@test.com',
    // Skip `vp install` inside `vp migrate` — snap tests don't need real installs
    VP_SKIP_INSTALL: '1',
    // make sure npm install global packages to the temporary directory
    NPM_CONFIG_PREFIX: path.join(tempTmpDir, NPM_GLOBAL_PREFIX_DIR),

    // A test case can override/unset environment variables above.
    // For example, VP_CLI_TEST/CI can be unset to test the real-world outputs.
    ...steps.env,
  };

  // Unset VP_NODE_VERSION to prevent `vp env use` session overrides
  // from leaking into snap tests.
  delete env['VP_NODE_VERSION'];

  // Unset VP_TOOL_RECURSION to prevent the shim recursion guard from
  // leaking into snap tests. When `pnpm` runs the test via the `vp` shim, vp
  // sets this marker before exec. Without clearing it, every npm/node command
  // in the test would bypass the managed shim and fall through to the system binary.
  delete env['VP_TOOL_RECURSION'];

  // Sometimes on Windows, the PATH variable is named 'Path'
  if ('Path' in env && !('PATH' in env)) {
    env['PATH'] = env['Path'];
    delete env['Path'];
  }
  // The node shim prepends ~/.vite-plus/js_runtime/node/VERSION/bin/ to PATH,
  // which leaks into this process. Strip internal vite-plus paths so the test
  // environment simulates a clean user PATH (only the shim bin dir + system paths).
  const vitePlusJsRuntime = path.join(env['VP_HOME'], 'js_runtime');
  env['PATH'] = [
    // Extend PATH to include the package's bin directory
    // --bin-dir overrides the default for cases like global CLI tests
    // where vp should resolve to the Rust binary instead of the Node.js script
    path.resolve(expandHome(binDir || 'bin')),
    ...env['PATH'].split(path.delimiter).filter((p) => !p.startsWith(vitePlusJsRuntime)),
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
