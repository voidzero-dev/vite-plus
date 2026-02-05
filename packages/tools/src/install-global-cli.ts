import { execSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const isWindows = process.platform === 'win32';

// Get repo root from script location (packages/tools/src/install-global-cli.ts -> repo root)
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../../..');

export function installGlobalCli() {
  // Detect if running directly or via tools dispatcher
  const isDirectInvocation = process.argv[1]?.endsWith('install-global-cli.ts');
  const args = process.argv.slice(isDirectInvocation ? 2 : 3);

  const { positionals, values } = parseArgs({
    allowPositionals: true,
    args,
    options: {
      tgz: {
        type: 'string',
        short: 't',
      },
    },
  });

  const binName = positionals[0];
  if (!binName || !['vp', 'vp-dev'].includes(binName)) {
    console.error('Usage: tool install-global-cli <vp|vp-dev> [--tgz <path>]');
    process.exit(1);
  }

  console.log(`Installing global CLI with bin name: ${binName}`);

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
    tempDir = mkdtempSync(path.join(os.tmpdir(), 'vite-plus-cli-'));

    // Use pnpm pack to create tarball
    // - Auto-resolves catalog: dependencies
    // - Includes binary (already in packages/global/bin/ after copy-vp-binary)
    execSync(`pnpm pack --pack-destination "${tempDir}"`, {
      cwd: path.join(repoRoot, 'packages/global'),
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
    // Set up environment for install script
    // Both vp and vp-dev use ~/.vite-plus-dev to avoid conflicting with release version
    const installDir = path.join(os.homedir(), '.vite-plus-dev');

    const env: Record<string, string> = {
      ...(process.env as Record<string, string>),
      VITE_PLUS_LOCAL_TGZ: tgzPath,
      VITE_PLUS_HOME: installDir,
      VITE_PLUS_VERSION: 'local-dev',
      CI: 'true',
    };

    // Run platform-specific install script (use absolute paths)
    const installScriptDir = path.join(repoRoot, 'packages/global');
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

    // Create wrapper scripts
    const binDir = path.join(installDir, 'bin');
    const currentBinDir = path.join(installDir, 'current', 'bin');

    // Create wrapper scripts to ensure VITE_PLUS_HOME is always set
    if (isWindows) {
      // On Windows, install.ps1 already creates bin/vp.cmd with VITE_PLUS_HOME set.
      // For 'vp-dev', we need to rename it to vp-dev.cmd.
      if (binName === 'vp-dev') {
        const vpCmd = path.join(binDir, 'vp.cmd');
        const vpDevCmd = path.join(binDir, 'vp-dev.cmd');
        if (existsSync(vpCmd)) {
          renameSync(vpCmd, vpDevCmd);
          console.log(`\nRenamed ${vpCmd} -> ${vpDevCmd}`);
        }
      }
      // For 'vp', bin/vp.cmd is already correct from install.ps1
    } else {
      // Unix: Rename vp -> vp-raw, then create a wrapper at vp
      // The wrapper sets VITE_PLUS_HOME and VITE_PLUS_SHIM_TOOL for shim detection
      const vpBinary = path.join(currentBinDir, 'vp');
      const vpRawBinary = path.join(currentBinDir, 'vp-raw');

      // Rename vp -> vp-raw (always replace to ensure latest binary)
      if (existsSync(vpBinary)) {
        if (existsSync(vpRawBinary)) {
          rmSync(vpRawBinary);
        }
        renameSync(vpBinary, vpRawBinary);
        console.log(`Renamed ${vpBinary} -> ${vpRawBinary}`);
      }

      // Create vp wrapper in current/bin/ that sets VITE_PLUS_HOME and calls vp-raw
      // Uses VITE_PLUS_SHIM_TOOL env var for shim detection (more portable than exec -a)
      const vpWrapperPath = path.join(currentBinDir, 'vp');
      const vpWrapperContent = `#!/bin/sh
VITE_PLUS_SHIM_TOOL="$(basename "$0")"
export VITE_PLUS_SHIM_TOOL
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp-raw" "$@"
`;
      writeFileSync(vpWrapperPath, vpWrapperContent);
      chmodSync(vpWrapperPath, 0o755);
      console.log(`Created wrapper: ${vpWrapperPath}`);

      // On Unix, create shell script wrappers
      if (binName === 'vp-dev') {
        // Remove the vp symlink to avoid confusion
        rmSync(path.join(binDir, 'vp'), { force: true });

        // Create vp-dev wrapper that points to current/bin/vp (the wrapper)
        const wrapperPath = path.join(binDir, 'vp-dev');
        const wrapperContent = `#!/bin/sh
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp" "$@"
`;
        writeFileSync(wrapperPath, wrapperContent);
        chmodSync(wrapperPath, 0o755);
        console.log(`\nCreated wrapper script: ${wrapperPath}`);
      }
      // For 'vp' on Unix, install.sh already creates the symlink to ../current/bin/vp
      // which now points to the wrapper script (which calls vp-raw)
    }

    // Patch env files for vp-dev: the shell function wrappers created by `vp env setup`
    // define vp() but in dev mode the binary is vp-dev, so we rename the functions
    if (binName === 'vp-dev') {
      const envPatches: Array<{ file: string; replacements: [string, string][] }> = [
        {
          file: 'env',
          replacements: [
            ['vp() {', 'vp-dev() {'],
            ['command vp ', 'command vp-dev '],
          ],
        },
        {
          file: 'env.fish',
          replacements: [
            ['function vp\n', 'function vp-dev\n'],
            ['command vp ', 'command vp-dev '],
          ],
        },
        {
          file: 'env.ps1',
          replacements: [['function vp {', 'function vp-dev {']],
        },
      ];

      for (const { file, replacements } of envPatches) {
        const filePath = path.join(installDir, file);
        if (existsSync(filePath)) {
          let content = readFileSync(filePath, 'utf-8');
          for (const [from, to] of replacements) {
            content = content.replaceAll(from, to);
          }
          writeFileSync(filePath, content);
          console.log(`Patched ${filePath} for vp-dev`);
        }
      }
    }
  } finally {
    // Cleanup temp dir only if we created it
    if (tempDir) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  }
}

// Allow running directly via: npx tsx install-global-cli.ts <args>
if (import.meta.main) {
  installGlobalCli();
}
