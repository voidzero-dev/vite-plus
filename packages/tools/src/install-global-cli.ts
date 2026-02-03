import { execSync } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';

const isWindows = process.platform === 'win32';

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
      cwd: 'packages/global',
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

    // Run platform-specific install script
    if (isWindows) {
      // Use pwsh (PowerShell Core) for better UTF-8 handling
      execSync(`pwsh -ExecutionPolicy Bypass -File .\\packages\\global\\install.ps1`, {
        stdio: 'inherit',
        env,
      });
    } else {
      execSync('bash ./packages/global/install.sh', {
        stdio: 'inherit',
        env,
      });
    }

    // Create wrapper scripts
    const binDir = path.join(installDir, 'bin');

    if (isWindows) {
      // On Windows, create bash script wrappers for Git Bash compatibility
      // (Git Bash doesn't execute .cmd files automatically)
      if (binName === 'vp-dev') {
        // Remove the vp.cmd to avoid confusion
        rmSync(path.join(binDir, 'vp.cmd'), { force: true });

        // Create vp-dev.cmd for cmd.exe/PowerShell
        const cmdPath = path.join(binDir, 'vp-dev.cmd');
        const cmdContent = `@echo off\r
set VITE_PLUS_HOME=${installDir}\r
"%VITE_PLUS_HOME%\\current\\bin\\vp.exe" %*\r
exit /b %ERRORLEVEL%\r
`;
        writeFileSync(cmdPath, cmdContent);

        // Create vp-dev bash script for Git Bash
        const bashPath = path.join(binDir, 'vp-dev');
        const bashContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp.exe" "$@"
`;
        writeFileSync(bashPath, bashContent);
        console.log(`\nCreated wrapper scripts: ${cmdPath}, ${bashPath}`);
      } else {
        // For 'vp', create bash script wrapper for Git Bash
        // (install.ps1 already creates vp.cmd for cmd.exe/PowerShell)
        const bashPath = path.join(binDir, 'vp');
        const bashContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp.exe" "$@"
`;
        writeFileSync(bashPath, bashContent);
        console.log(`\nCreated bash wrapper: ${bashPath}`);
      }
    } else {
      // On Unix, create shell script wrappers
      if (binName === 'vp-dev') {
        // Remove the vp symlink to avoid confusion
        rmSync(path.join(binDir, 'vp'), { force: true });

        // Create vp-dev wrapper that points directly to the binary
        const wrapperPath = path.join(binDir, 'vp-dev');
        const wrapperContent = `#!/bin/bash
export VITE_PLUS_HOME="${installDir}"
exec "$VITE_PLUS_HOME/current/bin/vp" "$@"
`;
        writeFileSync(wrapperPath, wrapperContent);
        chmodSync(wrapperPath, 0o755);
        console.log(`\nCreated wrapper script: ${wrapperPath}`);
      }
      // For 'vp' on Unix, install.sh already creates the symlink
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
