import { execFileSync, execSync, spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { appendFile, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import {
  VITEST_VERSION,
  VITE_PLUS_CORE_ALIAS_PREFIX,
} from '../packages/cli/src/utils/constants.ts';
import { isObjectValue, parseJsonObject } from '../packages/cli/src/utils/json-value.ts';
import { hasOwnKey } from '../packages/cli/src/utils/object.ts';
import vitePlusCorePkg from '../packages/core/package.json' with { type: 'json' };
import { patchNpmxVitestConfig } from './npmx.ts';
import { ecosystemCiDir, tgzDir, vitePlusCoreTgzVersion, vitePlusTgzVersion } from './paths.ts';
import repos from './repo.json' with { type: 'json' };
import { prepareWebdriverioProject } from './webdriverio.ts';

const project = process.argv[2];

if (!hasOwnKey(repos, project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const repoConfig = repos[project];
const repoRoot = join(ecosystemCiDir, project);
// Migrate and install always run at the clone root: `vp migrate` rejects
// workspace member targets (#2229). The workflow's `directory` matrix value
// only steers where the later project commands run (e.g. dify's `vp run ...`
// in web/).
const cli = process.env.VP_CLI_BIN ?? 'vp';

// The packed local build in tmp/tgz is served through a local npm registry
// (local-npm-registry.ts), so vp migrate pins and installs the checkout's
// own version through the standard registry code paths, with no `file:` specs.
const vitePlusVersion = vitePlusTgzVersion();
const vitePlusCoreVersion = vitePlusCoreTgzVersion();

const registryScript = join(
  import.meta.dirname,
  '..',
  'packages',
  'tools',
  'src',
  'local-npm-registry.ts',
);
// Detach the server so it can outlive this script on CI: the lockfiles
// written below reference its tarball URLs, and later workflow steps (the
// project's own vp commands) inherit the registry env via GITHUB_ENV.
// stderr must not inherit this process's streams: the detached server would
// hold the step's output pipe open after this script exits.
const registryServer = spawn(
  process.execPath,
  [registryScript, '--serve', '--packages-dir', tgzDir],
  {
    stdio: ['ignore', 'pipe', 'ignore'],
    detached: true,
  },
);
const handshake = await new Promise<string>((resolve, reject) => {
  let buffered = '';
  registryServer.stdout.on('data', (chunk: Buffer) => {
    buffered += chunk.toString();
    const newline = buffered.indexOf('\n');
    if (newline !== -1) {
      resolve(buffered.slice(0, newline));
    }
  });
  registryServer.on('error', reject);
  registryServer.on('exit', (code) => reject(new Error(`registry exited early (${code})`)));
});
const { registry, env } = parseJsonObject(handshake, 'registry handshake');
if (typeof registry !== 'string' || !isObjectValue(env)) {
  throw new Error('Invalid registry handshake');
}
const registryEnv: Record<string, string> = {};
for (const [key, value] of Object.entries(env)) {
  if (typeof value !== 'string') {
    throw new Error(`Invalid registry environment variable ${key}`);
  }
  registryEnv[key] = value;
}
console.log(`Serving local Vite+ packages at ${registry} (vite-plus@${vitePlusVersion})`);
// The server prints nothing after the handshake; release the pipe and the
// process handle so they don't keep this script's event loop alive after the
// installs below finish.
registryServer.stdout.destroy();
registryServer.unref();

if (process.env.GITHUB_ENV) {
  // Keep the registry reachable for the workflow's later steps.
  const lines = Object.entries(registryEnv)
    .map(([key, value]) => `${key}=${value}\n`)
    .join('');
  await appendFile(process.env.GITHUB_ENV, lines);
} else {
  process.on('exit', () => registryServer.kill());
}

await prepareWebdriverioProject(project, repoRoot, cli);

if (project === 'npmx.dev') {
  // The pinned workspace enables strict peer checks. Supply devframe's optional
  // cac peer explicitly so migration satisfies its ^7.0.0 requirement.
  const packagePath = join(repoRoot, 'package.json');
  const pkg = parseJsonObject(await readFile(packagePath, 'utf8'), packagePath);
  if (!isObjectValue(pkg.devDependencies)) {
    throw new Error('Expected npmx.dev to have devDependencies');
  }
  pkg.devDependencies.cac = '7.0.0';
  await writeFile(packagePath, `${JSON.stringify(pkg, null, 2)}\n`, 'utf8');
}

if (project === 'rollipop') {
  const oxfmtrc = await readFile(join(repoRoot, '.oxfmtrc.json'), 'utf8');
  await writeFile(
    join(repoRoot, '.oxfmtrc.json'),
    oxfmtrc.replace('      ["ts-equals-import"],\n', ''),
    'utf8',
  );
}

if (project === 'vinext') {
  // Vite 8.4 checks file access per environment. Stack frames returned over
  // HTTP must use the client environment's file access rules.
  execFileSync('git', ['apply', join(import.meta.dirname, 'vinext-vite-8.4.patch')], {
    cwd: repoRoot,
  });

  // vinext sets `minimumReleaseAge` (24h) which blocks fresh upstream upgrades
  // (e.g. oxc 0.129.0 published <24h ago). Disable it for the ecosystem run so
  // upgrade-deps PRs can install transitive deps that were just published.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf8');
  const patched = workspace.replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0');
  if (patched === workspace) {
    throw new Error(`vinext patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  await writeFile(workspacePath, patched, 'utf8');

  // vinext uses standalone `void` expressions to mark intentional reads.
  // tsrs5 removes `void` during autofix, then no-unused-expressions flags the reads.
  const viteConfigPath = join(repoRoot, 'vite.config.ts');
  const viteConfig = await readFile(viteConfigPath, 'utf8');
  const lintRule = '      "typescript/consistent-type-definitions": ["error", "type"],';
  if (!viteConfig.includes(lintRule)) {
    throw new Error(`vinext patch: expected TypeScript lint rule in ${viteConfigPath}`);
  }
  await writeFile(
    viteConfigPath,
    viteConfig.replace(
      lintRule,
      `${lintRule}\n      "typescript/no-meaningless-void-operator": "off",`,
    ),
    'utf8',
  );

  // oxlint 1.77 applies `.gitignore` to explicitly passed paths too
  // (oxc-project/oxc#25133). vinext's prefer-shared-utils rule test symlinks a
  // temp fixture directory into the repo and lints those files by path, and
  // `.gitignore` covers the link name, so oxlint now reports "No files found to
  // lint". Drop the ignore entry so the rule test keeps linting its fixtures.
  const gitignorePath = join(repoRoot, '.gitignore');
  const gitignore = await readFile(gitignorePath, 'utf8');
  const patchedGitignore = gitignore.replace(/^__lint_rule_fixtures__-\*$\n?/m, '');
  if (patchedGitignore === gitignore) {
    throw new Error(`vinext patch: \`__lint_rule_fixtures__-*\` not found in ${gitignorePath}`);
  }
  await writeFile(gitignorePath, patchedGitignore, 'utf8');
}

if (project === 'dify') {
  // pnpm 11 defaults `minimumReleaseAge` to 24 hours when the setting is
  // omitted, so keep dify's explicit opt-out instead of deleting it. Switch
  // this ephemeral fixture away from `resolutionMode: time-based` as well:
  // defining `minimumReleaseAge` there activates a resolution-policy path that
  // vp's bundled pnpm cannot handle.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf8');
  if (!/^minimumReleaseAge:/m.test(workspace)) {
    throw new Error(`dify patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  if (!/^resolutionMode:\s*time-based[ \t]*$/m.test(workspace)) {
    throw new Error(`dify patch: \`resolutionMode: time-based\` not found in ${workspacePath}`);
  }
  const patched = workspace
    .replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0')
    .replace(/^resolutionMode:\s*time-based[ \t]*$/m, 'resolutionMode: highest');
  await writeFile(workspacePath, patched, 'utf8');
}

if (project === 'nuxt-devtools') {
  // The fixture's lockfile is generated earlier in this trusted CI job against
  // the local registry. Trust that lockfile when package scripts invoke pnpm
  // again: the registry's unpublished 0.0.0 tarballs do not have npm trust
  // metadata, so a second supply-chain verification rejects them.
  //
  // Nuxt DevTools uses one YAML anchor for its Vite DevTools package family.
  // Align that source with vite-plus-core so pnpm resolves the core package,
  // kit, and optional integration peers as one compatible release family.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf8');
  const trustPolicy = 'trustPolicy: no-downgrade';
  if (!workspace.includes(trustPolicy)) {
    throw new Error(`nuxt-devtools patch: \`${trustPolicy}\` not found in ${workspacePath}`);
  }
  const viteDevtoolsVersionSource = /^([ \t]*vite-devtools:[ \t]+&vite-devtools[ \t]+)\S+[ \t]*$/m;
  if (!viteDevtoolsVersionSource.test(workspace)) {
    throw new Error(
      `nuxt-devtools patch: Vite DevTools version source not found in ${workspacePath}`,
    );
  }
  const viteDevtoolsVersion = vitePlusCorePkg.devDependencies['@vitejs/devtools'];
  const patched = workspace
    .replace(trustPolicy, `${trustPolicy}\ntrustLockfile: true`)
    .replace(
      viteDevtoolsVersionSource,
      (_line, prefix: string) => `${prefix}${viteDevtoolsVersion}`,
    );
  await writeFile(workspacePath, patched, 'utf8');
}

// Projects that already use vite-plus need VP_FORCE_MIGRATE=1 so
// vp migrate runs full dependency rewriting instead of skipping.
const forceFreshMigration = 'forceFreshMigration' in repoConfig && repoConfig.forceFreshMigration;

// Mirror VITE_PLUS_OVERRIDE_PACKAGES: pin `vitest` only. Vitest pins its
// internal dependencies; the migration aligns first-party browser providers.
//
// Coverage providers are intentionally NOT in the shipped override map (the
// product leaves them user-owned; the runtime guard fail-fasts on a skew). But
// this rig FORCE-INSTALLS the locally built vitest, and many ecosystem projects
// pin an older `@vitest/coverage-*` in their lockfile. Without alignment, the
// forced runner skews from the project's pinned provider and the guard
// aborts `vp test --coverage` — testing an incoherent combo no real install has.
// Pin the providers here so the E2E coverage step runs against a consistent
// runner+provider pair, exactly as a user who followed the guard's advice would.
const vitestOverrides = {
  vitest: VITEST_VERSION,
  '@vitest/coverage-v8': VITEST_VERSION,
  '@vitest/coverage-istanbul': VITEST_VERSION,
};

// E2E intentionally installs just-published toolchain packages (e.g.
// @oxlint/migrate during `vp migrate`, freshly bumped @oxc-project/runtime
// during `vp install`). Disable pnpm's minimumReleaseAge gate so a same-day
// publish does not fail with ERR_PNPM_NO_MATURE_MATCHING_VERSION. pnpm >= 10.6
// only reads the PNPM_CONFIG_* spelling; older pnpm reads the lowercase form.
//
// Projects that retain `resolutionMode: time-based` are the exception: defining
// a minimumReleaseAge (even 0, via any env spelling) activates pnpm's
// resolution-policy engine there, which vp's bundled pnpm cannot handle
// (ERR_PNPM_RESOLUTION_POLICY_VIOLATIONS_UNHANDLED, no
// handleResolutionPolicyViolations callback wired). Dify is patched to
// `resolutionMode: highest` above, so it follows the normal opt-out path.
const workspaceYamlPath = join(repoRoot, 'pnpm-workspace.yaml');
const timeBasedResolution =
  existsSync(workspaceYamlPath) &&
  /^resolutionMode:\s*time-based/m.test(readFileSync(workspaceYamlPath, 'utf8'));
const releaseAgeEnv = timeBasedResolution
  ? {}
  : {
      pnpm_config_minimum_release_age: '0',
      PNPM_CONFIG_MINIMUM_RELEASE_AGE: '0',
    };

const migrateEnv: NodeJS.ProcessEnv = {
  ...process.env,
  ...registryEnv,
  VP_OVERRIDE_PACKAGES: JSON.stringify({
    vite: `${VITE_PLUS_CORE_ALIAS_PREFIX}${vitePlusCoreVersion}`,
    ...vitestOverrides,
  }),
  // The vp binary was built before the pack step pinned the package versions,
  // so align the version migrate pins with the tgz the registry serves.
  VP_VERSION: vitePlusVersion,
  ...releaseAgeEnv,
};
if (forceFreshMigration) {
  migrateEnv.VP_FORCE_MIGRATE = '1';
}

execSync(`${cli} migrate --no-agent --no-interactive`, {
  cwd: repoRoot,
  stdio: 'inherit',
  env: migrateEnv,
});

if (project === 'vue-mini') {
  // Apply the assertion-types review item without changing matcher behavior.
  // Remove when the pinned setup declares both Vitest 5 type parameters.
  // https://vitest.dev/guide/migration/#assertion-types-expose-return-and-received-types
  const setupPath = join(repoRoot, 'vitest.setup.ts');
  const setup = await readFile(setupPath, 'utf8');
  const legacyAssertion = 'interface Assertion<T = any> extends CustomMatchers<T> {}';
  if (setup.split(legacyAssertion).length !== 2) {
    throw new Error('vue-mini patch: expected the pinned custom matcher declaration');
  }
  await writeFile(
    setupPath,
    setup.replace(legacyAssertion, 'interface Assertion<R, T> extends CustomMatchers<R> {}'),
    'utf8',
  );
}

if (project === 'dify') {
  // The happy-dom setup registers jest-dom matchers, but omits their Vitest
  // types. Its jest-dom 6 adapter still augments the old Assertion<T> interface;
  // use the v5 extension point so browser declarations cannot take precedence.
  // Remove when the pinned project provides a v5-compatible matcher augmentation.
  // https://vitest.dev/guide/migration/#assertion-types-expose-return-and-received-types
  const setupPath = join(repoRoot, 'web', 'vitest.setup.ts');
  const setup = await readFile(setupPath, 'utf8');
  const registration = "if (typeof expect.extend === 'function') {";
  if (
    setup.split(registration).length !== 2 ||
    setup.includes('interface Matchers<') ||
    !setup.includes('expect.extend(jestDomMatchers)')
  ) {
    throw new Error('dify patch: expected the pinned Jest DOM setup');
  }
  await writeFile(
    setupPath,
    setup.replace(
      registration,
      `// Match the jest-dom implementations registered below, not browser-mode matchers.
declare module 'vite-plus/test' {
  interface Matchers<R, T> extends jestDomMatchers.TestingLibraryMatchers<T, R> {}
}

${registration}`,
    ),
    'utf8',
  );
}

if (project === 'npmx.dev') {
  // Nuxt loads app modules before per-file mocks; use request interception.
  // https://vitest.dev/config/browser/importmapmocks
  const viteConfigPath = join(repoRoot, 'vite.config.ts');
  const viteConfig = await readFile(viteConfigPath, 'utf8');
  await writeFile(viteConfigPath, patchNpmxVitestConfig(viteConfig), 'utf8');
}

if (project === 'bun-vite-template') {
  // The pinned template runs Vitest only. Its JS setup file loads jest-dom's
  // Vitest runtime, but allowJs: false excludes that file from type checking.
  // Apply the migration report's manual type-entry repair for this fixture.
  const tsconfigPath = join(repoRoot, 'tsconfig.json');
  const tsconfig = parseJsonObject(await readFile(tsconfigPath, 'utf8'), tsconfigPath);
  const options = tsconfig.compilerOptions;
  if (
    !isObjectValue(options) ||
    !Array.isArray(options.types) ||
    !options.types.includes('vitest/globals') ||
    !options.types.includes('@testing-library/jest-dom')
  ) {
    throw new Error('bun-vite-template patch: expected the pinned Jest DOM type configuration');
  }
  options.types = options.types.map((type) =>
    type === '@testing-library/jest-dom' ? '@testing-library/jest-dom/vitest' : type,
  );
  await writeFile(tsconfigPath, `${JSON.stringify(tsconfig, null, 2)}\n`);
}

// Install through the local registry. `vp migrate` already pinned
// `vite-plus@<version>` in package.json exactly like a real migration, so no
// manual rewrite of the Vite+ version is needed.
execSync(`${cli} install --no-frozen-lockfile`, {
  cwd: repoRoot,
  stdio: 'inherit',
  env: { ...process.env, ...registryEnv, ...releaseAgeEnv },
});
