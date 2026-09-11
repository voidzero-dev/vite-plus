import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url));

export function resolveCargoTargetDir(configured: string | undefined): string {
  return path.resolve(repoRoot, configured || 'target');
}

export function resolveCargoArgs(args: string[]): string[] {
  const xwin = args.includes('--xwin');
  const cargoArgs = args.filter((arg) => arg !== '--xwin');
  if (xwin) {
    return ['xwin', 'build', ...cargoArgs];
  }
  return ['build', ...cargoArgs];
}

export function buildTrampoline(args: string[] = process.argv.slice(2)): void {
  const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
  execFileSync(cargo, resolveCargoArgs(args), {
    cwd: path.join(repoRoot, 'crates/vp_trampoline'),
    env: {
      ...process.env,
      CARGO_TARGET_DIR: resolveCargoTargetDir(process.env.CARGO_TARGET_DIR),
    },
    stdio: 'inherit',
  });
}

if (import.meta.main) {
  buildTrampoline();
}
