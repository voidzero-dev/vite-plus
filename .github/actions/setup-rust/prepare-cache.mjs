import { execFileSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function prepareRustCache({
  run = (args) => execFileSync('rustup', args, { encoding: 'utf8' }).trim(),
  env = process.env,
  home = homedir(),
} = {}) {
  const active = run(['show', 'active-toolchain']).split(/\s+/)[0];
  const installed = run(['toolchain', 'list', '--quiet']).split(/\s+/).filter(Boolean);
  if (!active || !installed.includes(active)) {
    throw new Error('The active Rust toolchain must be installed before cache setup');
  }

  // Cached jobs build with the workspace toolchain. rust-cache also hashes
  // unused toolchains from the runner image, which vary across image updates.
  // Keep the workspace compiler as the default before removing those extras.
  run(['default', active]);
  for (const toolchain of installed) {
    if (toolchain !== active) {
      run(['toolchain', 'uninstall', toolchain]);
    }
  }

  // GitHub runners leave these defaults unset; Namespace sets them explicitly.
  // rust-cache hashes nonempty environment variables, so use the same values.
  // Preserve custom paths, including Windows Dev Drives.
  return {
    CARGO_HOME: env.CARGO_HOME || join(home, '.cargo'),
    RUSTUP_HOME: env.RUSTUP_HOME || join(home, '.rustup'),
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const paths = prepareRustCache();
  appendFileSync(
    process.env.GITHUB_ENV,
    Object.entries(paths)
      .map(([key, value]) => `${key}=${value}\n`)
      .join(''),
  );
}
