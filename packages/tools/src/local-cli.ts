import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const isWindows = process.platform === 'win32';
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const cliDistDir = path.join(repoRoot, 'packages', 'cli', 'dist');
const cliBinPath = path.join(cliDistDir, 'bin.js');
const testCliPath = path.join(repoRoot, 'packages', 'test', 'dist', 'cli.js');
const localVpBinaryName = isWindows ? 'vp.exe' : 'vp';
const defaultLocalVpPath = path.join(repoRoot, 'target', 'debug', localVpBinaryName);
const viteRepoDir = path.join(repoRoot, 'vite');
const legacyViteRepoDir = path.join(repoRoot, 'rolldown-vite');
const rolldownRepoDir = path.join(repoRoot, 'rolldown');
const rolldownPackageDir = path.join(rolldownRepoDir, 'packages', 'rolldown');
const rolldownPackageJsonPath = path.join(rolldownPackageDir, 'package.json');
const rolldownSrcDir = path.join(rolldownPackageDir, 'src');
const toolBinPath = path.join(path.dirname(fileURLToPath(import.meta.url)), 'bin.js');
const buildHint = 'pnpm build:cli';
const bootstrapHint = 'pnpm bootstrap:dev';
const installHint = 'pnpm install:dev';
const pnpmExecPath = process.env.npm_execpath;
const pnpmBin = isWindows ? 'pnpm.cmd' : 'pnpm';
const cargoBin = isWindows ? 'cargo.exe' : 'cargo';
const requireFromRolldown = createRequire(rolldownPackageJsonPath);

type CommandOptions = {
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  hint?: string;
};

type LocalCliArtifacts = {
  vpPath: string;
  vpBinDir: string;
};

function failMissing(pathname: string, description: string): never {
  console.error(`Missing ${description}: ${pathname}`);
  console.error(`Run "${bootstrapHint}" from a fresh clone, or "${buildHint}" after setup.`);
  process.exit(1);
}

function getTargetDirs(): string[] {
  return [
    ...new Set(
      [process.env.CARGO_TARGET_DIR, path.join(repoRoot, 'target')].filter(
        (targetDir): targetDir is string => Boolean(targetDir),
      ),
    ),
  ];
}

function findLocalVpBinary(): string | null {
  const profiles = ['debug', 'release'];

  for (const targetDir of getTargetDirs()) {
    for (const profile of profiles) {
      const directPath = path.join(targetDir, profile, localVpBinaryName);
      if (existsSync(directPath)) {
        return directPath;
      }
    }

    try {
      for (const entry of readdirSync(targetDir).toSorted()) {
        for (const profile of profiles) {
          const nestedPath = path.join(targetDir, entry, profile, localVpBinaryName);
          if (existsSync(nestedPath)) {
            return nestedPath;
          }
        }
      }
    } catch {
      continue;
    }
  }

  return null;
}

function ensureLocalCliReady(options?: { needsTestCli?: boolean }): LocalCliArtifacts {
  if (!existsSync(cliBinPath)) {
    failMissing(cliBinPath, 'local CLI bundle');
  }
  const vpPath = findLocalVpBinary();
  if (!vpPath) {
    failMissing(defaultLocalVpPath, 'local vp binary');
  }
  if (options?.needsTestCli && !existsSync(testCliPath)) {
    failMissing(testCliPath, 'local test CLI bundle');
  }

  return {
    vpPath,
    vpBinDir: path.dirname(vpPath),
  };
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
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-android-arm64/package.json'];
      }
      if (process.arch === 'arm') {
        return ['@rolldown/binding-android-arm-eabi/package.json'];
      }
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
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-freebsd-arm64/package.json'];
      }
      if (process.arch === 'x64') {
        return ['@rolldown/binding-freebsd-x64/package.json'];
      }
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
      if (process.arch === 'ppc64') {
        return ['@rolldown/binding-linux-ppc64-gnu/package.json'];
      }
      if (process.arch === 'riscv64') {
        return [
          '@rolldown/binding-linux-riscv64-gnu/package.json',
          '@rolldown/binding-linux-riscv64-musl/package.json',
        ];
      }
      if (process.arch === 's390x') {
        return ['@rolldown/binding-linux-s390x-gnu/package.json'];
      }
      if (process.arch === 'x64') {
        return [
          '@rolldown/binding-linux-x64-gnu/package.json',
          '@rolldown/binding-linux-x64-musl/package.json',
        ];
      }
      return [];
    case 'win32':
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-win32-arm64-msvc/package.json'];
      }
      if (process.arch === 'ia32') {
        return ['@rolldown/binding-win32-ia32-msvc/package.json'];
      }
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

function hasRolldownPackagedBinding() {
  const candidates = rolldownBindingCandidates();
  if (candidates.length === 0) {
    return true;
  }

  for (const candidate of candidates) {
    try {
      requireFromRolldown.resolve(candidate);
      return true;
    } catch {
      continue;
    }
  }

  return false;
}

function materializeRolldownPackagedBindings() {
  for (const candidate of rolldownBindingCandidates()) {
    let packageJsonPath: string;
    try {
      packageJsonPath = requireFromRolldown.resolve(candidate);
    } catch {
      continue;
    }

    const packageDir = path.dirname(packageJsonPath);
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8')) as {
      files?: string[];
      main?: string;
    };
    const bindingFile =
      packageJson.main ?? packageJson.files?.find((file) => file.endsWith('.node'));
    if (!bindingFile) {
      continue;
    }

    const sourcePath = path.join(packageDir, bindingFile);
    const targetPath = path.join(rolldownSrcDir, path.basename(bindingFile));
    if (!existsSync(targetPath)) {
      copyFileSync(sourcePath, targetPath);
    }
  }
}

function ensureBuildWorkspaceReady() {
  if (!existsSync(viteRepoDir)) {
    console.error(`Missing local vite checkout: ${viteRepoDir}`);
    if (existsSync(legacyViteRepoDir)) {
      console.error(
        `Found legacy checkout at ${legacyViteRepoDir}. This repo now expects the upstream Vite checkout at ./vite.`,
      );
      console.error(`Run "${installHint}" to recreate the canonical layout.`);
    } else {
      console.error(
        `Run "${installHint}" to fetch the local upstream checkouts, or "${bootstrapHint}" to prepare and build the local CLI.`,
      );
    }
    process.exit(1);
  }

  if (!existsSync(rolldownRepoDir)) {
    console.error(`Missing local rolldown checkout: ${rolldownRepoDir}`);
    console.error(
      `Run "${installHint}" to fetch the local upstream checkouts, or "${bootstrapHint}" to prepare and build the local CLI.`,
    );
    process.exit(1);
  }

  if (!existsSync(rolldownPackageJsonPath) || !existsSync(rolldownSrcDir)) {
    console.error(`Incomplete local rolldown checkout: ${rolldownPackageDir}`);
    console.error(
      `Run "${installHint}" to fetch the local upstream checkouts, or "${bootstrapHint}" to prepare and build the local CLI.`,
    );
    process.exit(1);
  }
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
  const { vpPath } = ensureLocalCliReady({ needsTestCli: args[0] === 'test' });

  const result = spawnSync(vpPath, args, {
    cwd: process.cwd(),
    env: localCliEnv(),
    stdio: 'inherit',
  });
  exitWith(result);
}

export function runLocalGlobalSnapTest(args: string[]) {
  const { vpBinDir } = ensureLocalCliReady();

  const result = spawnSync(
    process.execPath,
    [
      toolBinPath,
      'snap-test',
      '--dir',
      'snap-tests-global',
      '--local-vp-bin-dir',
      vpBinDir,
      ...args,
    ],
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
  const hasPackagedBinding = hasRolldownPackagedBinding();
  if (!hasPackagedBinding) {
    runPnpmCommand(
      'Build rolldown native binding',
      ['--filter', 'rolldown', releaseRust ? 'build-binding:release' : 'build-binding'],
      {
        hint: 'If this fails, install "cmake" so rolldown can build its native binding from source.',
      },
    );
  } else {
    materializeRolldownPackagedBindings();
  }
  runPnpmCommand('Build rolldown JS glue', ['--filter', 'rolldown', 'build-node'], {
    hint: 'If this fails with a missing rolldown native binding, rerun "pnpm install:dev". If the error mentions "cmake", install cmake to build rolldown from source.',
  });
  runPnpmCommand(
    'Build vite rolled-up types',
    ['-C', 'vite', '--filter', 'vite', 'build-types-roll'],
    {
      hint: 'If this fails because vite dependencies are missing, rerun "pnpm install" from the repo root.',
    },
  );
  runPnpmCommand(
    'Type-check vite declarations',
    ['-C', 'vite', '--filter', 'vite', 'build-types-check'],
    {
      hint: 'If this fails because vite dependencies are missing, rerun "pnpm install" from the repo root.',
    },
  );
  runPnpmCommand('Build vite-plus core', ['--filter', '@voidzero-dev/vite-plus-core', 'build']);
  runPnpmCommand('Build vite-plus test', ['--filter', '@voidzero-dev/vite-plus-test', 'build']);
  runPnpmCommand('Build vite-plus prompts', [
    '--filter',
    '@voidzero-dev/vite-plus-prompts',
    'build',
  ]);
  runPnpmCommand('Build vite-plus CLI', ['--filter', 'vite-plus', 'build'], {
    env: releaseRust ? process.env : localBuildEnv,
  });
  runCommand('Build Rust CLI binaries', cargoBin, [
    'build',
    '-p',
    'vite_global_cli',
    '-p',
    'vite_trampoline',
    ...(releaseRust ? ['--release'] : []),
  ]);
}
