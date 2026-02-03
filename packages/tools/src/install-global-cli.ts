import { execSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
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

    // Rename the actual vp binary to vp-raw, then create a wrapper at vp
    // This ensures VITE_PLUS_HOME is always set when vp is invoked (including via shims)
    // The wrapper uses `exec -a "$0"` to preserve argv[0] for shim detection
    if (isWindows) {
      const vpExe = path.join(currentBinDir, 'vp.exe');
      const vpRawExe = path.join(currentBinDir, 'vp-raw.exe');

      // Rename vp.exe -> vp-raw.exe
      if (existsSync(vpExe) && !existsSync(vpRawExe)) {
        renameSync(vpExe, vpRawExe);
        console.log(`Renamed ${vpExe} -> ${vpRawExe}`);
      }

      // Create vp.cmd wrapper in current/bin/ that sets VITE_PLUS_HOME and calls vp-raw.exe
      const vpWrapperPath = path.join(currentBinDir, 'vp.cmd');
      const vpWrapperContent = `@echo off\r
set VITE_PLUS_HOME=${installDir}\r
"%~dp0vp-raw.exe" %*\r
exit /b %ERRORLEVEL%\r
`;
      writeFileSync(vpWrapperPath, vpWrapperContent);
      console.log(`Created wrapper: ${vpWrapperPath}`);

      // On Windows, create bash script wrappers for Git Bash compatibility
      // (Git Bash doesn't execute .cmd files automatically)
      if (binName === 'vp-dev') {
        // Remove the vp.cmd in bin/ to avoid confusion
        rmSync(path.join(binDir, 'vp.cmd'), { force: true });

        // Create vp-dev.cmd for cmd.exe/PowerShell
        const cmdPath = path.join(binDir, 'vp-dev.cmd');
        const cmdContent = `@echo off\r
set VITE_PLUS_HOME=${installDir}\r
"%VITE_PLUS_HOME%\\current\\bin\\vp.cmd" %*\r
exit /b %ERRORLEVEL%\r
`;
        writeFileSync(cmdPath, cmdContent);

        // Create vp-dev bash script for Git Bash
        const bashPath = path.join(binDir, 'vp-dev');
        const bashContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp.cmd" "$@"
`;
        writeFileSync(bashPath, bashContent);
        console.log(`\nCreated wrapper scripts: ${cmdPath}, ${bashPath}`);
      } else {
        // For 'vp', update bin/vp.cmd to call vp.cmd instead of vp.exe
        // (install.ps1 creates it pointing to vp.exe, but we renamed that to vp-raw.exe)
        const cmdPath = path.join(binDir, 'vp.cmd');
        const cmdContent = `@echo off\r
set VITE_PLUS_HOME=${installDir}\r
"%VITE_PLUS_HOME%\\current\\bin\\vp.cmd" %*\r
exit /b %ERRORLEVEL%\r
`;
        writeFileSync(cmdPath, cmdContent);

        // Also create bash script wrapper for Git Bash
        const bashPath = path.join(binDir, 'vp');
        const bashContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp.cmd" "$@"
`;
        writeFileSync(bashPath, bashContent);
        console.log(`\nCreated wrapper scripts: ${cmdPath}, ${bashPath}`);
      }
    } else {
      // Unix: Rename vp -> vp-raw, create wrapper
      const vpBinary = path.join(currentBinDir, 'vp');
      const vpRawBinary = path.join(currentBinDir, 'vp-raw');

      // Rename vp -> vp-raw
      if (existsSync(vpBinary) && !existsSync(vpRawBinary)) {
        renameSync(vpBinary, vpRawBinary);
        console.log(`Renamed ${vpBinary} -> ${vpRawBinary}`);
      }

      // Create vp wrapper in current/bin/ that sets VITE_PLUS_HOME and calls vp-raw
      // Uses `exec -a "$0"` to preserve argv[0] for shim detection (node, npm, npx)
      const vpWrapperPath = path.join(currentBinDir, 'vp');
      const vpWrapperContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec -a "$0" "$VITE_PLUS_HOME/current/bin/vp-raw" "$@"
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
        const wrapperContent = `#!/bin/bash
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
