import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { PackageManager } from '../types/index.ts';
import { runCommandSilently } from './command.ts';
import { readJsonFile } from './json.ts';
import { accent } from './terminal.ts';

/**
 * pnpm prints this prefix whenever it gates a dependency's build (install /
 * postinstall) script behind explicit approval. It appears both in the pnpm
 * >= 11 hard-error line (`[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts:
 * better-sqlite3@11.0.0, esbuild@0.25.0`) and the pnpm 10 warning box
 * (`Ignored build scripts: esbuild.`).
 */
const IGNORED_BUILDS_MARKER = 'Ignored build scripts:';

/** pnpm >= 11 turns the gated-builds warning into a hard exit-1 with this code. */
const IGNORED_BUILDS_ERROR_CODE = 'ERR_PNPM_IGNORED_BUILDS';

/** Box-drawing / list characters pnpm wraps the warning message in. */
const BOX_CHARS_AT_END = /[│|╮╯╰╭─\s]+$/u;
const BOX_CHARS = /[│|╮╯╰╭─]/gu;

export function isPnpmIgnoredBuildsError(output: string): boolean {
  return output.includes(IGNORED_BUILDS_ERROR_CODE);
}

/**
 * Strip a trailing `@version` from a (possibly scoped) package spec.
 * `better-sqlite3@11.0.0` -> `better-sqlite3`, `@scope/pkg@1.2.3` ->
 * `@scope/pkg`, `esbuild` -> `esbuild`.
 */
export function stripPackageVersion(spec: string): string {
  const at = spec.lastIndexOf('@');
  return at > 0 ? spec.slice(0, at) : spec;
}

/**
 * Parse the package names pnpm reports under "Ignored build scripts:" from
 * captured install output. Handles both the pnpm >= 11 single-line error and
 * the pnpm 10 boxed warning, strips version suffixes, and dedupes while
 * preserving first-seen order. Returns `[]` when the marker is absent.
 */
export function parseIgnoredBuilds(output: string): string[] {
  if (!output) {
    return [];
  }
  const markerIndex = output.indexOf(IGNORED_BUILDS_MARKER);
  if (markerIndex === -1) {
    return [];
  }
  // Only the marker's own line carries the package list; the "Run pnpm
  // approve-builds" hint and box borders live on other lines.
  let segment = output.slice(markerIndex + IGNORED_BUILDS_MARKER.length);
  const newlineIndex = segment.indexOf('\n');
  if (newlineIndex !== -1) {
    segment = segment.slice(0, newlineIndex);
  }
  segment = segment.replace(BOX_CHARS_AT_END, '').replace(/\.$/u, '').trim();
  if (!segment) {
    return [];
  }

  const names: string[] = [];
  const seen = new Set<string>();
  for (const rawToken of segment.split(',')) {
    const token = rawToken.replace(BOX_CHARS, '').trim();
    if (!token) {
      continue;
    }
    const name = stripPackageVersion(token);
    if (!name || seen.has(name)) {
      continue;
    }
    seen.add(name);
    names.push(name);
  }
  return names;
}

/**
 * Parse the package names from `bun pm untrusted` output. bun does not hard-fail
 * on gated builds; after install it lists each blocked package on its own line
 * as `./node_modules/<name> @<version>` (scoped: `@scope/pkg`, nested:
 * `./node_modules/a/node_modules/b`). The name after the last `node_modules/`
 * is what `bun pm trust` expects. Returns deduped names in first-seen order;
 * `[]` when nothing is blocked.
 */
export function parseBunUntrusted(output: string): string[] {
  if (!output) {
    return [];
  }
  const names: string[] = [];
  const seen = new Set<string>();
  for (const line of output.split('\n')) {
    const marker = line.trimEnd().lastIndexOf('node_modules/');
    if (marker === -1) {
      continue;
    }
    const rest = line.trimEnd().slice(marker + 'node_modules/'.length);
    // A package entry is `<name> @<version>`; the name never contains a space
    // (so the postinstall detail lines that mention paths are skipped).
    const match = rest.match(/^(@?[^\s]+) @[^\s]+$/u);
    if (!match) {
      continue;
    }
    const name = match[1];
    if (seen.has(name)) {
      continue;
    }
    seen.add(name);
    names.push(name);
  }
  return names;
}

/**
 * Collect the names a project directly depends on (the dependencies it can
 * meaningfully approve). peerDependencies are intentionally excluded: they are
 * not installed into the project's own tree.
 */
export function collectDirectDependencyNames(
  pkg: Record<string, unknown> | undefined,
): Set<string> {
  const names = new Set<string>();
  if (!pkg) {
    return names;
  }
  for (const field of ['dependencies', 'devDependencies', 'optionalDependencies'] as const) {
    const deps = pkg[field];
    if (deps && typeof deps === 'object') {
      for (const name of Object.keys(deps)) {
        names.add(name);
      }
    }
  }
  return names;
}

export function filterToDirectDependencies(ignored: string[], direct: Set<string>): string[] {
  return ignored.filter((name) => direct.has(name));
}

/** Package managers that gate build scripts and expose an approval workflow. */
const GATED_BUILD_PACKAGE_MANAGERS: ReadonlySet<PackageManager> = new Set([
  PackageManager.pnpm,
  PackageManager.bun,
]);

/**
 * Narrow a package manager's gated builds down to the ones worth surfacing
 * during `vp create`: packages the generated project depends on directly.
 * Transitive gated builds (e.g. `esbuild` pulled in by Vite) are noise the user
 * did not choose, so they are dropped. Returns `[]` for package managers that
 * do not gate build scripts (npm, yarn classic), since there is nothing to
 * approve.
 */
export function resolveApproveBuildTargets(
  projectDir: string,
  pendingBuilds: string[] | undefined,
  packageManager: PackageManager | undefined,
): string[] {
  if (
    !packageManager ||
    !GATED_BUILD_PACKAGE_MANAGERS.has(packageManager) ||
    !pendingBuilds ||
    pendingBuilds.length === 0
  ) {
    return [];
  }
  let pkg: Record<string, unknown>;
  try {
    pkg = readJsonFile(path.join(projectDir, 'package.json'));
  } catch {
    return [];
  }
  const direct = collectDirectDependencyNames(pkg);
  const deduped = [...new Set(pendingBuilds)];
  return filterToDirectDependencies(deduped, direct);
}

/**
 * Enumerate the packages whose build scripts a package manager gated during the
 * install, as raw names (still unfiltered by direct dependency).
 *
 * - pnpm reports them in its install output (a hard exit-1 on v11), so its names
 *   are parsed there and passed in via `pendingBuildsFromInstall`.
 * - bun exits 0 and only prints a count, so `bun pm untrusted` is queried here.
 *
 * Other package managers run build scripts by default and return `[]`.
 */
export async function detectGatedBuilds(
  installCwd: string,
  packageManager: PackageManager | undefined,
  pendingBuildsFromInstall: string[] | undefined,
): Promise<string[]> {
  if (packageManager === PackageManager.pnpm) {
    return pendingBuildsFromInstall ?? [];
  }
  if (packageManager === PackageManager.bun) {
    const { exitCode, stdout, stderr } = await runCommandSilently({
      command: process.env.VP_CLI_BIN ?? 'vp',
      args: ['exec', 'bun', 'pm', 'untrusted'],
      cwd: installCwd,
      envs: process.env,
    });
    if (exitCode !== 0) {
      return [];
    }
    return parseBunUntrusted(`${stdout.toString()}\n${stderr.toString()}`);
  }
  return [];
}

function makeSpinner(interactive: boolean, silent: boolean) {
  if (silent) {
    return { start: () => {}, stop: () => {}, message: () => {} };
  }
  if (interactive) {
    return prompts.spinner();
  }
  return {
    start: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    stop: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    message: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
  };
}

function lastLines(text: string, count: number): string {
  const lines = text.split('\n');
  return lines.slice(-count).join('\n');
}

function printApproveBuildsGuidance(targets: string[]): void {
  prompts.log.warn(`Build scripts were not run for: ${accent(targets.join(', '))}.`);
  prompts.log.info(
    `These dependencies may not work until built. Run ${accent('vp pm approve-builds')} in the ` +
      `project to approve them, or re-create with ${accent('--approve-builds')}.`,
  );
}

async function runApproveBuilds(
  cwd: string,
  packages: string[],
  interactive: boolean,
  silent: boolean,
): Promise<void> {
  const spinner = makeSpinner(interactive, silent);
  spinner.start(`Building ${packages.join(', ')}...`);
  const { exitCode, stdout, stderr } = await runCommandSilently({
    command: process.env.VP_CLI_BIN ?? 'vp',
    args: ['pm', 'approve-builds', ...packages],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Built ${packages.join(', ')}`);
    return;
  }
  spinner.stop(`Build failed for ${packages.join(', ')}`);
  const output = `${stdout.toString()}\n${stderr.toString()}`.trim();
  if (output) {
    prompts.log.info(lastLines(output, 20));
  }
  // approve-builds records the approval in pnpm config even when the build
  // itself fails, so a later `vp install` retries the build once the toolchain
  // is fixed.
  prompts.log.warn(
    `Build scripts failed for ${accent(packages.join(', '))}. They were approved; fix the ` +
      `build toolchain and run ${accent('vp install')} to retry.`,
  );
}

export interface ApproveBuildsOptions {
  /** Directory the package manager ran in (where `node_modules` lives). */
  cwd: string;
  /** Direct-dependency packages with gated build scripts (already filtered). */
  targets: string[];
  interactive: boolean;
  /** `--approve-builds`: approve and build every target without prompting. */
  autoApprove: boolean;
  silent?: boolean;
}

/**
 * Surface pnpm's gated build scripts after a `vp create` install and let the
 * user act on them:
 * - `--approve-builds`: approve + build every target, no prompt.
 * - interactive: a default-off multiselect so each package is approved
 *   individually (pnpm gates them for security, so nothing is opt-in by
 *   default).
 * - non-interactive: print guidance pointing at `vp pm approve-builds`.
 */
export async function approveBuilds(options: ApproveBuildsOptions): Promise<void> {
  const { cwd, targets, interactive, autoApprove, silent = false } = options;
  if (targets.length === 0) {
    return;
  }

  let selected: string[];
  if (autoApprove) {
    selected = targets;
  } else if (interactive) {
    const answer = await prompts.multiselect<string>({
      message:
        'These dependencies have build scripts (e.g. native builds) that pnpm did not run. ' +
        'Select which to approve and build:',
      options: targets.map((name) => ({ value: name, label: name })),
      initialValues: [],
      required: false,
    });
    if (prompts.isCancel(answer)) {
      printApproveBuildsGuidance(targets);
      return;
    }
    selected = answer;
  } else {
    printApproveBuildsGuidance(targets);
    return;
  }

  if (selected.length === 0) {
    printApproveBuildsGuidance(targets);
    return;
  }

  await runApproveBuilds(cwd, selected, interactive, silent);
}
