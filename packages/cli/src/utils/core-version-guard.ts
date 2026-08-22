/**
 * Version-skew guard for the project's `vite` alias.
 *
 * `vp create` / `vp migrate` scaffold two entries that must move in lockstep:
 * the `vite-plus` dependency and the `vite` alias
 * (`npm:@voidzero-dev/vite-plus-core@<same version>`). A dependency bot sees
 * two unrelated packages and bumps them in separate PRs, so a project can end
 * up running a CLI/core pairing that was never published together (#2356).
 * The skew is silent: `vp build`/`vp dev`/`vp test` execute the CLI's own
 * core dependency, while plugins and configs that `import 'vite'` load the
 * project's aliased copy at the other version. Fail fast instead, so a
 * mismatched bot PR fails CI before the pairing ships.
 *
 * The expected version is the running CLI package's own version
 * ({@link CLI_PACKAGE_VERSION}), never an env-derived one: `VP_VERSION` can
 * linger from the installer session or arrive injected by a parent `vp`
 * process, and preview builds publish CLI and core from one commit with equal
 * versions, so the package version is correct for every flow.
 */

import { CLI_PACKAGE_VERSION, VITE_PLUS_CORE_PACKAGE_NAME } from './constants.ts';
import { detectPackageMetadata } from './package.ts';

export const SKIP_CORE_VERSION_CHECK_ENV = 'VP_SKIP_CORE_VERSION_CHECK';

/**
 * Throw when the project's aliased core version differs from the version the
 * CLI expects. A no-op when no aliased core is installed.
 *
 * Exported for unit testing.
 */
export function assertCoreVersionMatch(
  installedVersion: string | null | undefined,
  expectedVersion: string,
): void {
  if (installedVersion && installedVersion !== expectedVersion) {
    // Keep every version inside a `@voidzero-dev/vite-plus-core@<x>` context:
    // the PTY snapshot redactor masks the CLI's own version only in that form
    // (a bare `vite-plus@<x>` stays verbatim and would churn every release).
    throw new Error(
      `The project's \`vite\` alias resolves to ${VITE_PLUS_CORE_PACKAGE_NAME}@${installedVersion}, ` +
        `but this vite-plus CLI requires ${VITE_PLUS_CORE_PACKAGE_NAME}@${expectedVersion}: the two ` +
        `packages are published in lockstep and other pairings are untested. A dependency ` +
        `bot usually causes this by updating vite-plus and the \`vite\` alias in separate ` +
        `PRs. Update the \`vite\` alias to npm:${VITE_PLUS_CORE_PACKAGE_NAME}@${expectedVersion} ` +
        `where it is declared (catalog, overrides, resolutions, or dependencies), or run ` +
        `\`vp migrate\` to realign it. Set ${SKIP_CORE_VERSION_CHECK_ENV}=1 to skip this check.`,
    );
  }
}

/**
 * Orchestrates the guard: honor the escape hatch, read what `vite` resolves
 * to from the command's directory (the copy plugins and configs import), and
 * assert it against the running CLI's version. A project on real Vite, or
 * with no `vite` installed, passes.
 *
 * The `expectedVersion` parameter exists for unit tests; production callers
 * use the default.
 */
export function checkCoreVersionMatch(
  projectDir: string = process.cwd(),
  expectedVersion: string = CLI_PACKAGE_VERSION,
): void {
  if (process.env[SKIP_CORE_VERSION_CHECK_ENV]) {
    return;
  }
  const installed = detectPackageMetadata(projectDir, 'vite');
  assertCoreVersionMatch(
    installed && installed.name === VITE_PLUS_CORE_PACKAGE_NAME ? installed.version : null,
    expectedVersion,
  );
}

const checkedDirs = new Set<string>();

/**
 * Memoized wrapper for the resolver path. The `vite`/`test` resolvers run
 * once per intercepted script command, so a `vp run` across a large workspace
 * would repeat the same read; one check per execution directory suffices.
 * The directory comes from the Rust side (the task cwd), because retargeted
 * runs (`defaultPackage`, `vp run -r`) execute in a package dir while the
 * Node process cwd stays at the invocation root.
 */
export function checkCoreVersionMatchOnce(projectDir: string = process.cwd()): void {
  if (checkedDirs.has(projectDir)) {
    return;
  }
  checkedDirs.add(projectDir);
  checkCoreVersionMatch(projectDir);
}
