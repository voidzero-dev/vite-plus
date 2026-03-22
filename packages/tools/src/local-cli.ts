import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const isWindows = process.platform === 'win32';
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const cliDistDir = path.join(repoRoot, 'packages', 'cli', 'dist');
const cliBinPath = path.join(cliDistDir, 'bin.js');
const testCliPath = path.join(repoRoot, 'packages', 'test', 'dist', 'cli.js');
const localVpPath = path.join(repoRoot, 'target', 'debug', isWindows ? 'vp.exe' : 'vp');
const localVpBinDir = path.dirname(localVpPath);
const viteRepoDir = path.join(repoRoot, 'vite');
const legacyViteRepoDir = path.join(repoRoot, 'rolldown-vite');
const toolBinPath = path.join(path.dirname(fileURLToPath(import.meta.url)), 'bin.js');
const buildHint = 'pnpm build:cli';
const pnpmExecPath = process.env.npm_execpath;
const pnpmBin = isWindows ? 'pnpm.cmd' : 'pnpm';
const cargoBin = isWindows ? 'cargo.exe' : 'cargo';
const requireFromRolldown = createRequire(
  path.join(repoRoot, 'rolldown', 'packages', 'rolldown', 'package.json'),
);

type CommandOptions = {
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  hint?: string;
};

function failMissing(pathname: string, description: string): never {
  console.error(`Missing ${description}: ${pathname}`);
  console.error(`Run "${buildHint}" first.`);
  process.exit(1);
}

function ensureLocalCliReady(options?: { needsTestCli?: boolean }) {
  if (!existsSync(cliBinPath)) {
    failMissing(cliBinPath, 'local CLI bundle');
  }
  if (!existsSync(localVpPath)) {
    failMissing(localVpPath, 'local debug vp binary');
  }
  if (options?.needsTestCli && !existsSync(testCliPath)) {
    failMissing(testCliPath, 'local test CLI bundle');
  }
}

function localCliEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    VITE_GLOBAL_CLI_JS_SCRIPTS_DIR: cliDistDir,
  };
}

function rolldownBindingCandidates() {
  switch (process.platform) {
    case 'android':
      if (process.arch === 'arm64') return ['@rolldown/binding-android-arm64/package.json'];
      if (process.arch === 'arm') return ['@rolldown/binding-android-arm-eabi/package.json'];
      return [];
    case 'darwin':
      if (process.arch === 'arm64') {
        return [
          '@rolldown/binding-darwin-universal/package.json',
          '@rolldown/binding-darwin-arm64/package.json',
        ];
      }
      if (process.arch === 'x64') {
        return [
          '@rolldown/binding-darwin-universal/package.json',
          '@rolldown/binding-darwin-x64/package.json',
        ];
      }
      return [];
    case 'freebsd':
      if (process.arch === 'arm64') return ['@rolldown/binding-freebsd-arm64/package.json'];
      if (process.arch === 'x64') return ['@rolldown/binding-freebsd-x64/package.json'];
      return [];
    case 'linux':
      if (process.arch === 'arm') {
        return [
          '@rolldown/binding-linux-arm-gnueabihf/package.json',
          '@rolldown/binding-linux-arm-musleabihf/package.json',
        ];
      }
      if (process.arch === 'arm64') {
        return [
          '@rolldown/binding-linux-arm64-gnu/package.json',
          '@rolldown/binding-linux-arm64-musl/package.json',
        ];
      }
      if (process.arch === 'loong64') {
        return [
          '@rolldown/binding-linux-loong64-gnu/package.json',
          '@rolldown/binding-linux-loong64-musl/package.json',
        ];
      }
      if (process.arch === 'ppc64') return ['@rolldown/binding-linux-ppc64-gnu/package.json'];
      if (process.arch === 'riscv64') {
        return [
          '@rolldown/binding-linux-riscv64-gnu/package.json',
          '@rolldown/binding-linux-riscv64-musl/package.json',
        ];
      }
      if (process.arch === 's390x') return ['@rolldown/binding-linux-s390x-gnu/package.json'];
      if (process.arch === 'x64') {
        return [
          '@rolldown/binding-linux-x64-gnu/package.json',
          '@rolldown/binding-linux-x64-musl/package.json',
        ];
      }
      return [];
    case 'win32':
      if (process.arch === 'arm64') return ['@rolldown/binding-win32-arm64-msvc/package.json'];
      if (process.arch === 'ia32') return ['@rolldown/binding-win32-ia32-msvc/package.json'];
      if (process.arch === 'x64') {
        return [
          '@rolldown/binding-win32-x64-msvc/package.json',
          '@rolldown/binding-win32-x64-gnu/package.json',
        ];
      }
      return [];
    default:
      return [];
  }
}

function ensureBuildWorkspaceReady() {
  if (!existsSync(viteRepoDir)) {
    console.error(`Missing local vite checkout: ${viteRepoDir}`);
    if (existsSync(legacyViteRepoDir)) {
      console.error(
        `Found legacy checkout at ${legacyViteRepoDir}. This repo now expects the upstream Vite checkout at ./vite.`,
      );
      console.error(
        'Run "node packages/tools/src/index.ts sync-remote" to recreate the canonical layout.',
      );
    } else {
      console.error(
        'Run "node packages/tools/src/index.ts sync-remote" to fetch the local upstream checkouts required for development.',
      );
    }
    process.exit(1);
  }

  const candidates = rolldownBindingCandidates();
  if (candidates.length === 0) {
    return;
  }

  for (const candidate of candidates) {
    try {
      requireFromRolldown.resolve(candidate);
      return;
    } catch {
      continue;
    }
  }

  console.error('Missing local rolldown native binding dependency.');
  console.error('Run "pnpm install" from the repo root to install workspace optional dependencies.');
  console.error('If your environment cannot download the prebuilt binding, install "cmake" to build rolldown from source.');
  process.exit(1);
}

function runCommand(step: string, command: string, args: string[], options: CommandOptions = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: options.env ?? process.env,
    stdio: 'inherit',
  });

  if (!result.error && result.status === 0) {
    return;
  }

  console.error(`\n${step} failed.`);
  if (result.error) {
    console.error(result.error.message);
  }
  if (options.hint) {
    console.error(options.hint);
  }
  process.exit(result.status ?? 1);
}

function runPnpmCommand(step: string, args: string[], options: CommandOptions = {}) {
  const baseArgs = pnpmExecPath ? [pnpmExecPath] : [];
  const command = pnpmExecPath ? process.execPath : pnpmBin;

  runCommand(step, command, [...baseArgs, ...args], options);
}

function exitWith(result: ReturnType<typeof spawnSync>): never {
  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  process.exit(result.status ?? 1);
}

export function runLocalCli(args: string[]) {
  ensureLocalCliReady({ needsTestCli: args[0] === 'test' });

  const result = spawnSync(localVpPath, args, {
    cwd: process.cwd(),
    env: localCliEnv(),
    stdio: 'inherit',
  });
  exitWith(result);
}

export function runLocalGlobalSnapTest(args: string[]) {
  ensureLocalCliReady();

  const result = spawnSync(
    process.execPath,
    [toolBinPath, 'snap-test', '--dir', 'snap-tests-global', '--bin-dir', localVpBinDir, ...args],
    {
      cwd: process.cwd(),
      env: localCliEnv(),
      stdio: 'inherit',
    },
  );
  exitWith(result);
}

export function runBuildLocalCli(args: string[]) {
  const releaseRust = args.includes('--release-rust');
  const localBuildEnv = {
    ...process.env,
    VITE_PLUS_CLI_DEBUG: '1',
  };

  mkdirSync(path.join(repoRoot, 'tmp'), { recursive: true });
  ensureBuildWorkspaceReady();

  runPnpmCommand('Build @rolldown/pluginutils', ['--filter', '@rolldown/pluginutils', 'build']);
  runPnpmCommand('Build rolldown JS glue', ['--filter', 'rolldown', 'build-node'], {
    hint: 'If this fails with a missing rolldown native binding, rerun "pnpm install". If the error mentions "cmake", install cmake to build rolldown from source.',
  });
  runPnpmCommand('Build vite rolled-up types', ['-C', 'vite', '--filter', 'vite', 'build-types-roll'], {
    hint: 'If this fails because vite dependencies are missing, rerun "pnpm install" from the repo root.',
  });
  runPnpmCommand('Type-check vite declarations', ['-C', 'vite', '--filter', 'vite', 'build-types-check'], {
    hint: 'If this fails because vite dependencies are missing, rerun "pnpm install" from the repo root.',
  });
  runPnpmCommand('Build vite-plus core', ['--filter', '@voidzero-dev/vite-plus-core', 'build']);
  runPnpmCommand('Build vite-plus test', ['--filter', '@voidzero-dev/vite-plus-test', 'build']);
  runPnpmCommand('Build vite-plus prompts', ['--filter', '@voidzero-dev/vite-plus-prompts', 'build']);
  runPnpmCommand('Build vite-plus CLI', ['--filter', 'vite-plus', 'build'], {
    env: releaseRust ? process.env : localBuildEnv,
  });
  runCommand(
    'Build Rust CLI binaries',
    cargoBin,
    ['build', '-p', 'vite_global_cli', '-p', 'vite_trampoline', ...(releaseRust ? ['--release'] : [])],
  );
}
