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
export const CORE_VERSION_MISMATCH_ERROR_KIND = 'core-version-mismatch';

export class CoreVersionMismatchError extends Error {
  override readonly name = 'CoreVersionMismatchError';
}

export interface CoreVersionResolverError {
  errorKind: typeof CORE_VERSION_MISMATCH_ERROR_KIND;
  errorMessage: string;
}

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
    throw new CoreVersionMismatchError(
      `Your \`vite\` alias uses ${VITE_PLUS_CORE_PACKAGE_NAME}@${installedVersion}.\n` +
        `This Vite+ CLI requires ${VITE_PLUS_CORE_PACKAGE_NAME}@${expectedVersion}.\n\n` +
        `Choose a fix:\n` +
        `- Update the \`vite\` alias to npm:${VITE_PLUS_CORE_PACKAGE_NAME}@${expectedVersion}.\n` +
        `- Run \`vp migrate\`.\n\n` +
        `To skip this check, set ${SKIP_CORE_VERSION_CHECK_ENV}=1.`,
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
export function checkCoreVersionMatchOnce(
  projectDir: string = process.cwd(),
  expectedVersion: string = CLI_PACKAGE_VERSION,
): void {
  if (checkedDirs.has(projectDir)) {
    return;
  }
  checkCoreVersionMatch(projectDir, expectedVersion);
  checkedDirs.add(projectDir);
}

/** Return a tagged error that Rust can distinguish from resolver failures. */
export function checkCoreVersionMatchForResolver(
  projectDir: string = process.cwd(),
  expectedVersion: string = CLI_PACKAGE_VERSION,
): CoreVersionResolverError | undefined {
  try {
    checkCoreVersionMatchOnce(projectDir, expectedVersion);
    return undefined;
  } catch (error) {
    if (error instanceof CoreVersionMismatchError) {
      return {
        errorKind: CORE_VERSION_MISMATCH_ERROR_KIND,
        errorMessage: error.message,
      };
    }
    throw error;
  }
}
