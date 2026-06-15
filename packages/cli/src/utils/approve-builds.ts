import path from 'node:path';
import { stripVTControlCharacters } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { PackageManager } from '../types/index.ts';
import { runCommandSilently } from './command.ts';
import { readJsonFile, writeJsonFile } from './json.ts';
import { getSilentSpinner, getSpinner } from './spinner.ts';
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

/** Box-drawing / list characters pnpm wraps the pnpm-10 warning message in. */
const BOX_CHARS = /[│|╮╯╰╭─]/gu;
/** Non-global form, for testing whether a line is a box-bordered continuation. */
const BOX_LINE = /[│|╮╯╰╭─]/u;

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
 * Collect the name `extract` pulls from each item, dropping empties and
 * duplicates while preserving first-seen order. The three install-output
 * parsers (pnpm / bun / yarn) differ only in how a name is read from each token
 * or line; this captures their shared dedupe-in-order loop.
 */
function dedupeNames<T>(
  items: Iterable<T>,
  extract: (item: T) => string | null | undefined,
): string[] {
  const names: string[] = [];
  const seen = new Set<string>();
  for (const item of items) {
    const name = extract(item);
    if (!name || seen.has(name)) {
      continue;
    }
    seen.add(name);
    names.push(name);
  }
  return names;
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
  // Strip ANSI/VT codes first: colorized pnpm output (e.g. FORCE_COLOR in CI)
  // wraps the warning box and package list in control sequences that would
  // otherwise cling to the trailing package name and break the match.
  const clean = stripVTControlCharacters(output);
  const markerIndex = clean.indexOf(IGNORED_BUILDS_MARKER);
  if (markerIndex === -1) {
    return [];
  }
  // Collect the marker's line plus any box-wrapped continuation lines (pnpm 10
  // word-wraps long lists inside its warning box). Stop at the "Run ...
  // approve-builds" hint, or the first line that is neither the marker line nor
  // a box-bordered continuation (pnpm 11's blank separator, or trailing install
  // output like "Done in 171ms").
  const lines = clean.slice(markerIndex + IGNORED_BUILDS_MARKER.length).split('\n');
  const listLines: string[] = [];
  for (const line of lines) {
    if (listLines.length > 0 && (/approve-builds/u.test(line) || !BOX_LINE.test(line))) {
      break;
    }
    listLines.push(line);
  }
  // Box borders become separators; the pnpm-10 form ends the list with a period.
  const segment = listLines
    .join(' ')
    .replace(BOX_CHARS, ' ')
    .replace(/[.\s]+$/u, '')
    .trim();
  if (!segment) {
    return [];
  }

  return dedupeNames(segment.split(','), (rawToken) => {
    const token = rawToken.trim();
    return token ? stripPackageVersion(token) : null;
  });
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
  return dedupeNames(output.split('\n'), (rawLine) => {
    const line = rawLine.trim();
    // A blocked-package entry is a path line (`./node_modules/<name> @<version>`).
    // Require that exact shape so the indented `» [postinstall]: ...` detail
    // lines — which may themselves contain a `node_modules/` path — are skipped.
    if (!line.startsWith('./node_modules/') && !line.startsWith('node_modules/')) {
      return null;
    }
    const rest = line.slice(line.lastIndexOf('node_modules/') + 'node_modules/'.length);
    const match = rest.match(/^(@?[^\s]+) @[^\s]+$/u);
    return match ? match[1] : null;
  });
}

const YARN_DISABLED_BUILDS_MARKER = 'lists build scripts, but all build scripts have been disabled';

/**
 * Yarn (Berry) gates build scripts when `enableScripts` is false: each gated
 * package is reported on its own line as `<descriptor> lists build scripts, but
 * all build scripts have been disabled` (e.g. `core-js@npm:3.39.0 lists build
 * scripts...`). Yarn does not fail the install. Returns deduped package names.
 */
export function parseYarnDisabledBuilds(output: string): string[] {
  if (!output) {
    return [];
  }
  return dedupeNames(output.split('\n'), (rawLine) => {
    const line = stripVTControlCharacters(rawLine);
    const markerIndex = line.indexOf(YARN_DISABLED_BUILDS_MARKER);
    if (markerIndex === -1) {
      return null;
    }
    // The descriptor is the last whitespace-delimited token before the marker
    // (yarn descriptors never contain spaces), e.g. `core-js@npm:3.39.0`. Skip
    // yarn's optional virtual-peer hash, which trails the descriptor:
    // `svelte-preprocess@npm:6.0.3 [f4825] lists build scripts...`.
    const tokens = line
      .slice(0, markerIndex)
      .trim()
      .split(/\s+/u)
      .filter((token) => token && !/^\[[0-9a-f]+\]$/u.test(token));
    const descriptor = tokens.pop() ?? '';
    return yarnDescriptorName(descriptor);
  });
}

/**
 * Extract the package name from a yarn descriptor by dropping the trailing
 * `@<range>`: `core-js@npm:3.39.0` -> `core-js`, `@scope/pkg@npm:1.0.0` ->
 * `@scope/pkg`.
 */
function yarnDescriptorName(descriptor: string): string {
  const match = descriptor.match(/^(@[^@/]+\/[^@]+|[^@]+)@/u);
  return match ? match[1] : descriptor;
}

/**
 * Parse the gated build-script package names from an install log, dispatching on
 * the package manager: pnpm prints `Ignored build scripts:`, yarn prints
 * `... build scripts have been disabled`. bun is not parsed here (its blocked
 * packages are queried separately via `bun pm untrusted`).
 */
export function parseInstallGatedBuilds(
  output: string,
  packageManager: PackageManager | undefined,
): string[] {
  if (packageManager === PackageManager.pnpm) {
    return parseIgnoredBuilds(output);
  }
  if (packageManager === PackageManager.yarn) {
    return parseYarnDisabledBuilds(output);
  }
  return [];
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
      for (const [name, spec] of Object.entries(deps as Record<string, unknown>)) {
        names.add(name);
        // An `npm:` alias installs under the aliased package's real name, which
        // is what the package manager reports as the gated build (e.g.
        // `"sqlite": "npm:better-sqlite3@1.0.0"` is reported as `better-sqlite3`).
        if (typeof spec === 'string' && spec.startsWith('npm:')) {
          const aliased = stripPackageVersion(spec.slice('npm:'.length));
          if (aliased) {
            names.add(aliased);
          }
        }
      }
    }
  }
  return names;
}

export function filterToDirectDependencies(ignored: string[], direct: Set<string>): string[] {
  return ignored.filter((name) => direct.has(name));
}

/**
 * pnpm gained positional `approve-builds <pkg>` in pnpm 11; pnpm 10 only accepts
 * `--all` (and otherwise opens an interactive picker), so a non-interactive
 * positional approve there silently does nothing. When the version is unknown,
 * assume a modern pnpm (vp provisions 11+).
 */
export function pnpmSupportsPositionalApprove(version: string | undefined): boolean {
  if (!version) {
    return true;
  }
  const major = Number.parseInt(version, 10);
  return Number.isNaN(major) || major >= 11;
}

/** Package managers that gate build scripts and expose an approval workflow. */
const GATED_BUILD_PACKAGE_MANAGERS: ReadonlySet<PackageManager> = new Set([
  PackageManager.pnpm,
  PackageManager.bun,
  PackageManager.yarn,
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
 * - pnpm and yarn report them in their install output, so the names are parsed
 *   there (see {@link parseInstallGatedBuilds}) and passed in via
 *   `pendingBuildsFromInstall`.
 * - bun exits 0 and only prints a count, so `bun pm untrusted` is queried here.
 *
 * Other package managers run build scripts by default and return `[]`.
 */
export async function detectGatedBuilds(
  installCwd: string,
  packageManager: PackageManager | undefined,
  pendingBuildsFromInstall: string[] | undefined,
): Promise<string[]> {
  if (packageManager === PackageManager.pnpm || packageManager === PackageManager.yarn) {
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

function lastLines(text: string, count: number): string {
  const lines = text.split('\n');
  return lines.slice(-count).join('\n');
}

function printApproveBuildsGuidance(
  targets: string[],
  packageManager: PackageManager | undefined,
): void {
  prompts.log.warn(`Build scripts were not run for: ${accent(targets.join(', '))}.`);
  // yarn has no `approve-builds` command, so point at its own workflow instead.
  if (packageManager === PackageManager.yarn) {
    prompts.log.info(
      `These dependencies may not work until built. Enable them in the workspace root ` +
        `package.json (${accent('dependenciesMeta.<pkg>.built: true')}) and reinstall, or ` +
        `re-create with ${accent('--approve-builds')}.`,
    );
    return;
  }
  // bun's `pm approve-builds` is a no-op without explicit names (it just prints
  // "requires package names"), so spell them out. pnpm's runs an interactive
  // picker when called bare, so it doesn't need them.
  const command =
    packageManager === PackageManager.bun
      ? `vp pm approve-builds ${targets.join(' ')}`
      : 'vp pm approve-builds';
  prompts.log.info(
    `These dependencies may not work until built. Run ${accent(command)} in the ` +
      `project to approve them, or re-create with ${accent('--approve-builds')}.`,
  );
}

/**
 * Run a `vp` build/approval command and report the outcome through a spinner.
 * On failure the approval has still been recorded (pnpm/bun config or yarn's
 * `dependenciesMeta`), so the retry hint points back at `vp install`. Returns
 * `true` when the command succeeded, `false` when the build exited non-zero.
 */
async function runBuildAndReport(
  args: string[],
  cwd: string,
  packages: string[],
  interactive: boolean,
  silent: boolean,
  extraEnv?: Record<string, string>,
): Promise<boolean> {
  const spinner = silent ? getSilentSpinner() : getSpinner(interactive);
  spinner.start(`Building ${packages.join(', ')}...`);
  const { exitCode, stdout, stderr } = await runCommandSilently({
    command: process.env.VP_CLI_BIN ?? 'vp',
    args,
    cwd,
    envs: extraEnv ? { ...process.env, ...extraEnv } : process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Built ${packages.join(', ')}`);
    return true;
  }
  spinner.stop(`Build failed for ${packages.join(', ')}`);
  const output = `${stdout.toString()}\n${stderr.toString()}`.trim();
  if (output) {
    prompts.log.info(lastLines(output, 20));
  }
  prompts.log.warn(
    `Build scripts failed for ${accent(packages.join(', '))}. They were approved; fix the ` +
      `build toolchain and run ${accent('vp install')} to retry.`,
  );
  return false;
}

/**
 * Mark each package as build-allowed in yarn's `dependenciesMeta[<pkg>].built`,
 * preserving existing metadata. Guards against a non-object container or
 * per-package value so a hand-authored scalar doesn't corrupt package.json.
 * Mutates and returns `pkg`.
 */
export function addYarnBuiltDependenciesMeta(
  pkg: Record<string, unknown>,
  packages: string[],
): Record<string, unknown> {
  const existing = pkg.dependenciesMeta;
  const meta: Record<string, unknown> =
    existing && typeof existing === 'object' ? { ...(existing as Record<string, unknown>) } : {};
  for (const name of packages) {
    const current = meta[name];
    meta[name] = {
      ...(current && typeof current === 'object' ? (current as Record<string, unknown>) : {}),
      built: true,
    };
  }
  pkg.dependenciesMeta = meta;
  return pkg;
}

/**
 * Approve gated builds for yarn. Unlike pnpm/bun, yarn has no `approve-builds`
 * command: a package's build is enabled by setting `dependenciesMeta[name].built`
 * to true in package.json, after which a reinstall runs its build script.
 *
 * The metadata is written to the install root's manifest (`installCwd`), not the
 * created package: yarn only honors `dependenciesMeta.<pkg>.built` from the
 * workspace root, so a child-package entry is ignored and the build never runs.
 */
async function approveYarnBuilds(
  installCwd: string,
  packages: string[],
  interactive: boolean,
  silent: boolean,
): Promise<boolean> {
  const pkgPath = path.join(installCwd, 'package.json');
  let pkg: Record<string, unknown>;
  try {
    pkg = readJsonFile(pkgPath);
  } catch {
    printApproveBuildsGuidance(packages, PackageManager.yarn);
    return true;
  }
  writeJsonFile(pkgPath, addYarnBuiltDependenciesMeta(pkg, packages));
  // Writing `dependenciesMeta` changes the manifest, so the reinstall has to be
  // allowed to update the lockfile. Yarn Berry enables immutable installs by
  // default under CI and would otherwise fail with YN0028.
  return runBuildAndReport(['install'], installCwd, packages, interactive, silent, {
    YARN_ENABLE_IMMUTABLE_INSTALLS: 'false',
  });
}

export interface ApproveBuildsOptions {
  /** Directory the package manager ran in (where `node_modules` lives). */
  cwd: string;
  /**
   * Directory whose package.json declares the gated direct deps. Same as `cwd`
   * for a standalone project; the created package for a monorepo. (yarn records
   * `dependenciesMeta` at the install root in `cwd`, not here.)
   */
  projectDir: string;
  packageManager: PackageManager | undefined;
  /** Resolved package-manager version, used to gate version-specific behavior. */
  packageManagerVersion?: string;
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
 *
 * Returns `false` only when an approved build actually ran and failed (so a
 * non-interactive `--approve-builds` caller can surface a non-zero exit);
 * approving nothing or printing guidance returns `true`.
 */
export async function approveBuilds(options: ApproveBuildsOptions): Promise<boolean> {
  const {
    cwd,
    packageManager,
    packageManagerVersion,
    targets,
    interactive,
    autoApprove,
    silent = false,
  } = options;
  if (targets.length === 0) {
    return true;
  }

  let selected: string[];
  if (autoApprove) {
    selected = targets;
  } else if (interactive) {
    const answer = await prompts.multiselect<string>({
      message:
        'These dependencies have build scripts (e.g. native builds) that were not run. ' +
        'Select which to approve and build:',
      options: targets.map((name) => ({ value: name, label: name })),
      initialValues: [],
      required: false,
    });
    // Cancelling or selecting nothing both mean "approve nothing" -> guidance.
    selected = prompts.isCancel(answer) ? [] : answer;
  } else {
    selected = [];
  }

  if (selected.length === 0) {
    printApproveBuildsGuidance(targets, packageManager);
    return true;
  }

  if (packageManager === PackageManager.yarn) {
    return approveYarnBuilds(cwd, selected, interactive, silent);
  }
  if (
    packageManager === PackageManager.pnpm &&
    !pnpmSupportsPositionalApprove(packageManagerVersion)
  ) {
    // pnpm < 11 can't approve individual packages: its only non-interactive
    // option is `--all`, which would also build the transitive scripts we
    // deliberately leave at pnpm's defaults. So we don't auto-approve here;
    // point the user at `vp pm approve-builds` (pnpm's interactive picker). For
    // a non-interactive `--approve-builds` we couldn't honor the request, so
    // report failure (non-zero exit) instead of a silent no-op.
    printApproveBuildsGuidance(selected, packageManager);
    return !(autoApprove && !interactive);
  }
  return runBuildAndReport(
    ['pm', 'approve-builds', ...selected],
    cwd,
    selected,
    interactive,
    silent,
  );
}
