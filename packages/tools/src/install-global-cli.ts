import { execSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
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
    const installDir = path.join(os.homedir(), '.vite-plus');

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
    // install.sh/install.ps1 already creates the correct symlinks and wrappers for vp
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
