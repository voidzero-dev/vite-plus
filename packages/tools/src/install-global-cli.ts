import { execSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const isWindows = process.platform === 'win32';
const LOCAL_DEV_PREFIX = 'local-dev';
const pad2 = (n: number) => n.toString().padStart(2, '0');

function localDevVersion(): string {
  const now = new Date();
  const date = `${now.getFullYear()}${pad2(now.getMonth() + 1)}${pad2(now.getDate())}`;
  const time = `${pad2(now.getHours())}${pad2(now.getMinutes())}${pad2(now.getSeconds())}`;
  return `${LOCAL_DEV_PREFIX}-${date}-${time}`;
}

// Get repo root from script location (packages/tools/src/install-global-cli.ts -> repo root)
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../../..');

export function installGlobalCli() {
  // Detect if running directly or via tools dispatcher
  const isDirectInvocation = process.argv[1]?.endsWith('install-global-cli.ts');
  const args = process.argv.slice(isDirectInvocation ? 2 : 3);

  const { values } = parseArgs({
    allowPositionals: false,
    args,
    options: {
      tgz: {
        type: 'string',
        short: 't',
      },
    },
  });

  console.log('Installing global CLI: vp');

  let tempDir: string | undefined;
  let tgzPath: string;

  if (values.tgz) {
    // Use provided tgz file directly
    tgzPath = path.resolve(values.tgz);
    if (!existsSync(tgzPath)) {
      console.error(`Error: tgz file not found: ${tgzPath}`);
      process.exit(1);
    }
    console.log(`Using provided tgz: ${tgzPath}`);
  } else {
    // Create temp directory for pnpm pack output
    tempDir = mkdtempSync(path.join(os.tmpdir(), 'vite-plus-'));

    // Use pnpm pack to create tarball
    // - Auto-resolves catalog: dependencies
    execSync(`pnpm pack --pack-destination "${tempDir}"`, {
      cwd: path.join(repoRoot, 'packages/cli'),
      stdio: 'inherit',
    });

    // Find the generated tgz file (name includes version)
    const tgzFile = readdirSync(tempDir).find((f) => f.endsWith('.tgz'));
    if (!tgzFile) {
      throw new Error('pnpm pack did not create a .tgz file');
    }
    tgzPath = path.join(tempDir, tgzFile);
  }

  try {
    const installDir = path.join(os.homedir(), '.vite-plus');

    // Locate the Rust vp binary (built by cargo or copied by CI)
    const binaryName = isWindows ? 'vp.exe' : 'vp';
    const binaryPath = findVpBinary(binaryName);
    if (!binaryPath) {
      console.error(`Error: vp binary not found in ${path.join(repoRoot, 'target')}`);
      console.error('Run "cargo build -p vite_global_cli --release" first.');
      process.exit(1);
    }

    // On Windows, the trampoline shim binary is required for creating shims.
    // Validate it exists beside the chosen vp.exe to avoid mismatched artifacts.
    if (isWindows) {
      const shimPath = path.join(path.dirname(binaryPath), 'vp-shim.exe');
      if (!existsSync(shimPath)) {
        console.error(`Error: vp-shim.exe not found at ${shimPath}`);
        console.error('Build it with: cargo build -p vite_trampoline --release');
        process.exit(1);
      }
    }

    const localDevVer = localDevVersion();

    // Clean up old local-dev directories to avoid accumulation
    if (existsSync(installDir)) {
      for (const entry of readdirSync(installDir)) {
        if (entry.startsWith(LOCAL_DEV_PREFIX)) {
          try {
            rmSync(path.join(installDir, entry), { recursive: true, force: true });
          } catch (err) {
            console.warn(`Warning: failed to remove old ${entry}: ${(err as Error).message}`);
          }
        }
      }
    }

    const env: Record<string, string> = {
      ...(process.env as Record<string, string>),
      VITE_PLUS_LOCAL_TGZ: tgzPath,
      VITE_PLUS_LOCAL_BINARY: binaryPath,
      VITE_PLUS_HOME: installDir,
      VITE_PLUS_VERSION: localDevVer,
      CI: 'true',
      // Skip vp install in install.sh — we handle deps ourselves:
      // - Local dev: symlink monorepo node_modules
      // - CI (--tgz): rewrite @voidzero-dev/* deps to file: protocol and npm install
      VITE_PLUS_SKIP_DEPS_INSTALL: '1',
    };

    // Run platform-specific install script (use absolute paths)
    const installScriptDir = path.join(repoRoot, 'packages/cli');
    if (isWindows) {
      // Use pwsh (PowerShell Core) for better UTF-8 handling
      const ps1Path = path.join(installScriptDir, 'install.ps1');
      execSync(`pwsh -ExecutionPolicy Bypass -File "${ps1Path}"`, {
        stdio: 'inherit',
        env,
      });
    } else {
      const shPath = path.join(installScriptDir, 'install.sh');
      execSync(`bash "${shPath}"`, {
        stdio: 'inherit',
        env,
      });
    }

    // Set up node_modules for local dev by rewriting workspace deps to file: protocol
    // and running pnpm install. Production installs use `vp install` in install.sh directly.
    const versionDir = path.join(installDir, localDevVer);
    if (values.tgz) {
      installCiDeps(versionDir, tgzPath);
    } else {
      setupLocalDevDeps(versionDir);
    }
  } finally {
    // Cleanup temp dir only if we created it
    if (tempDir) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  }
}

// Find the vp binary in the target directory.
// Checks target/release/ first (local builds), then target/<triple>/release/ (cross-compiled CI builds).
function findVpBinary(binaryName: string) {
  // 1. Direct release build: target/release/vp
  const directPath = path.join(repoRoot, 'target', 'release', binaryName);
  if (existsSync(directPath)) {
    return directPath;
  }

  // 2. Cross-compiled build: target/<triple>/release/vp (CI builds with --target)
  const targetDir = path.join(repoRoot, 'target');
  if (existsSync(targetDir)) {
    for (const entry of readdirSync(targetDir)) {
      const crossPath = path.join(targetDir, entry, 'release', binaryName);
      if (existsSync(crossPath)) {
        return crossPath;
      }
    }
  }

  return null;
}

/**
 * Install dependencies for CI by generating a wrapper package.json with file: protocol
 * references to the main tgz and sibling @voidzero-dev/* tgz files, then running npm install.
 */
function installCiDeps(versionDir: string, mainTgzPath: string) {
  const tgzDir = path.dirname(mainTgzPath);

  // Extract vite-plus's package.json from the tgz to find @voidzero-dev/* deps
  // On Windows, use the system tar (bsdtar) which handles Windows paths natively.
  // Git Bash's GNU tar misinterprets drive letters (D:, C:) as remote host references,
  // affecting both the archive path and the -C directory argument.
  const tar = isWindows ? `"${process.env.SystemRoot}\\System32\\tar.exe"` : 'tar';
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'vp-deps-'));
  try {
    execSync(`${tar} xzf "${mainTgzPath}" -C "${tempDir}" --strip-components=1 package.json`, {
      stdio: 'pipe',
    });
  } catch {
    // If extracting just package.json fails, extract everything
    execSync(`${tar} xzf "${mainTgzPath}" -C "${tempDir}" --strip-components=1`, {
      stdio: 'pipe',
    });
  }
  const vitePlusPkg = JSON.parse(readFileSync(path.join(tempDir, 'package.json'), 'utf-8'));
  rmSync(tempDir, { recursive: true, force: true });

  // Build wrapper deps: vite-plus from tgz + @voidzero-dev/* from sibling tgz files
  const wrapperDeps: Record<string, string> = {
    'vite-plus': `file:${mainTgzPath}`,
  };

  const vitePlusDeps: Record<string, string> = vitePlusPkg.dependencies ?? {};
  for (const [name, version] of Object.entries(vitePlusDeps)) {
    if (!name.startsWith('@voidzero-dev/')) {
      continue;
    }
    // @voidzero-dev/vite-plus-core@0.0.0 -> voidzero-dev-vite-plus-core-0.0.0.tgz
    const tgzName = name.replace('@', '').replace('/', '-') + `-${version}.tgz`;
    const tgzFilePath = path.join(tgzDir, tgzName);
    if (existsSync(tgzFilePath)) {
      wrapperDeps[name] = `file:${tgzFilePath}`;
      console.log(`  ${name}: ${version} -> file:${tgzFilePath}`);
    } else {
      console.warn(`Warning: tgz not found for ${name}@${version}: ${tgzFilePath}`);
    }
  }

  const wrapperPkg = {
    name: 'vp-global',
    version: '0.0.0',
    private: true,
    dependencies: wrapperDeps,
  };

  writeFileSync(path.join(versionDir, 'package.json'), JSON.stringify(wrapperPkg, null, 2) + '\n');

  execSync('npm install --no-audit --no-fund --legacy-peer-deps', {
    cwd: versionDir,
    stdio: 'inherit',
  });
}

/**
 * Set up dependencies for local dev by symlinking into node_modules.
 *
 * Creates node_modules/vite-plus → packages/cli (source) and symlinks
 * transitive deps from packages/cli/node_modules into version_dir/node_modules.
 */
function setupLocalDevDeps(versionDir: string) {
  const nodeModulesDir = path.join(versionDir, 'node_modules');
  rmSync(nodeModulesDir, { recursive: true, force: true });
  mkdirSync(nodeModulesDir, { recursive: true });

  // Symlink node_modules/vite-plus → packages/cli (source)
  const cliDir = path.join(repoRoot, 'packages', 'cli');
  const symlinkType = isWindows ? 'junction' : 'dir';
  symlinkSync(cliDir, path.join(nodeModulesDir, 'vite-plus'), symlinkType);

  // Symlink transitive deps from packages/cli/node_modules
  const cliNodeModules = path.join(cliDir, 'node_modules');
  if (!existsSync(cliNodeModules)) {
    return;
  }

  for (const entry of readdirSync(cliNodeModules)) {
    if (entry === '.pnpm' || entry === '.modules.yaml') {
      continue;
    }
    const src = path.join(cliNodeModules, entry);
    const dest = path.join(nodeModulesDir, entry);
    if (!existsSync(dest)) {
      // Handle scoped packages (@scope/) by creating parent dir
      if (entry.startsWith('@')) {
        mkdirSync(dest, { recursive: true });
        for (const sub of readdirSync(src)) {
          symlinkSync(path.join(src, sub), path.join(dest, sub), symlinkType);
        }
      } else {
        symlinkSync(src, dest, symlinkType);
      }
    }
  }
}

// Allow running directly via: npx tsx install-global-cli.ts <args>
if (import.meta.main) {
  installGlobalCli();
}
