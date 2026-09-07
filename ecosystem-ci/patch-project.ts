import { execSync, spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { appendFile, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { VITEST_VERSION } from '../packages/cli/src/utils/constants.ts';
import vitePlusCorePkg from '../packages/core/package.json' with { type: 'json' };
import { ecosystemCiDir, tgzDir, vitePlusTgzVersion } from './paths.ts';
import repos from './repo.json' with { type: 'json' };

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const repoRoot = join(ecosystemCiDir, project);
const repoConfig = repos[project as keyof typeof repos];
// Migrate and install always run at the clone root: `vp migrate` rejects
// workspace member targets (#2229). The workflow's `directory` matrix value
// only steers where the later project commands run (e.g. dify's `vp run ...`
// in web/).
const cli = process.env.VP_CLI_BIN ?? 'vp';

// The packed local build in tmp/tgz is served through a local npm registry
// (local-npm-registry.ts), so vp migrate pins and installs the checkout's
// own version through the standard registry code paths, with no `file:` specs.
const vitePlusVersion = vitePlusTgzVersion();

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
const registryInfo = await new Promise<{ registry: string; env: Record<string, string> }>(
  (resolve, reject) => {
    let buffered = '';
    registryServer.stdout.on('data', (chunk: Buffer) => {
      buffered += chunk.toString();
      const newline = buffered.indexOf('\n');
      if (newline !== -1) {
        resolve(JSON.parse(buffered.slice(0, newline)));
      }
    });
    registryServer.on('error', reject);
    registryServer.on('exit', (code) => reject(new Error(`registry exited early (${code})`)));
  },
);
console.log(
  `Serving local Vite+ packages at ${registryInfo.registry} (vite-plus@${vitePlusVersion})`,
);
// The server prints nothing after the handshake; release the pipe and the
// process handle so they don't keep this script's event loop alive after the
// installs below finish.
registryServer.stdout.destroy();
registryServer.unref();

if (process.env.GITHUB_ENV) {
  // Keep the registry reachable for the workflow's later steps.
  const lines = Object.entries(registryInfo.env)
    .map(([key, value]) => `${key}=${value}\n`)
    .join('');
  await appendFile(process.env.GITHUB_ENV, lines);
} else {
  process.on('exit', () => registryServer.kill());
}

if (project === 'rollipop') {
  const oxfmtrc = await readFile(join(repoRoot, '.oxfmtrc.json'), 'utf-8');
  await writeFile(
    join(repoRoot, '.oxfmtrc.json'),
    oxfmtrc.replace('      ["ts-equals-import"],\n', ''),
    'utf-8',
  );
}

if (project === 'vinext') {
  // vinext sets `minimumReleaseAge` (24h) which blocks fresh upstream upgrades
  // (e.g. oxc 0.129.0 published <24h ago). Disable it for the ecosystem run so
  // upgrade-deps PRs can install transitive deps that were just published.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf-8');
  const patched = workspace.replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0');
  if (patched === workspace) {
    throw new Error(`vinext patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  await writeFile(workspacePath, patched, 'utf-8');

  // The single in-process `integration` project runs serially and its ISR
  // revalidation test sits right at the 30s ceiling under CI load (observed
  // 26.8s on green main runs, 30.0s here) — a borderline timeout, not a real
  // regression (the vitest runner is byte-identical across this bump). Give it
  // headroom so the ecosystem run isn't flaky.
  const viteConfigPath = join(repoRoot, 'vite.config.ts');
  const viteConfig = await readFile(viteConfigPath, 'utf-8');
  const testTimeout = 'testTimeout: 30000';
  const reactRulesAnchor = '"react/rules-of-hooks": "error",';
  if (!viteConfig.includes(testTimeout)) {
    throw new Error(`vinext patch: \`testTimeout: 30000\` not found in ${viteConfigPath}`);
  }
  if (!viteConfig.includes(reactRulesAnchor)) {
    throw new Error(`vinext patch: React lint rules not found in ${viteConfigPath}`);
  }
  // Oxlint 1.79 enables split React Compiler rules by default. Keep the new
  // diagnostics opt-in for this pinned ecosystem fixture.
  const patchedConfig = viteConfig
    .replace(testTimeout, 'testTimeout: 60000')
    .replace(
      reactRulesAnchor,
      [
        reactRulesAnchor,
        '      "react/globals": "off",',
        '      "react/refs": "off",',
        '      "react/set-state-in-effect": "off",',
      ].join('\n'),
    );
  await writeFile(viteConfigPath, patchedConfig, 'utf-8');

  // Oxlint 1.79 runs no-redeclare in ES modules. The declarations intentionally
  // use TypeScript interface/class merging, so extend their existing disables.
  const documentPath = join(repoRoot, 'packages/vinext/src/shims/document.tsx');
  const document = await readFile(documentPath, 'utf-8');
  const declarationMergeDisable =
    '// oxlint-disable-next-line typescript/consistent-type-definitions, typescript/no-unsafe-declaration-merging';
  if (document.split(declarationMergeDisable).length !== 3) {
    throw new Error(`vinext patch: declaration merge directives not found in ${documentPath}`);
  }
  const patchedDocument = document.replaceAll(
    declarationMergeDisable,
    '// oxlint-disable-next-line eslint/no-redeclare, typescript/consistent-type-definitions, typescript/no-unsafe-declaration-merging',
  );
  await writeFile(documentPath, patchedDocument, 'utf-8');

  // Oxlint 1.79 reports bare underscore parameters. Give this intentionally
  // unused parameter a descriptive underscore-prefixed name.
  const appRscHandlerTestPath = join(repoRoot, 'tests/app-rsc-handler.test.ts');
  const appRscHandlerTest = await readFile(appRscHandlerTestPath, 'utf-8');
  const unusedMiddlewareParameter = '(_: { nextUrl: URL })';
  const patchedAppRscHandlerTest = appRscHandlerTest.replace(
    unusedMiddlewareParameter,
    '(_request: { nextUrl: URL })',
  );
  if (patchedAppRscHandlerTest === appRscHandlerTest) {
    throw new Error(
      `vinext patch: unused middleware parameter not found in ${appRscHandlerTestPath}`,
    );
  }
  await writeFile(appRscHandlerTestPath, patchedAppRscHandlerTest, 'utf-8');

  // Oxlint 1.79 checks comments for irregular whitespace. Escape the glob
  // separator instead of retaining its invisible zero-width character.
  const trailingSlashTestPath = join(repoRoot, 'tests/app-route-handler-trailing-slash.test.ts');
  const trailingSlashTest = await readFile(trailingSlashTestPath, 'utf-8');
  const patchedTrailingSlashTest = trailingSlashTest.replace(
    'app/**\u200b/route.ts',
    'app/**\\/route.ts',
  );
  if (patchedTrailingSlashTest === trailingSlashTest) {
    throw new Error(`vinext patch: zero-width separator not found in ${trailingSlashTestPath}`);
  }
  await writeFile(trailingSlashTestPath, patchedTrailingSlashTest, 'utf-8');

  // oxlint 1.77 applies `.gitignore` to explicitly passed paths too
  // (oxc-project/oxc#25133). vinext's prefer-shared-utils rule test symlinks a
  // temp fixture directory into the repo and lints those files by path, and
  // `.gitignore` covers the link name, so oxlint now reports "No files found to
  // lint". Drop the ignore entry so the rule test keeps linting its fixtures.
  const gitignorePath = join(repoRoot, '.gitignore');
  const gitignore = await readFile(gitignorePath, 'utf-8');
  const patchedGitignore = gitignore.replace(/^__lint_rule_fixtures__-\*$\n?/m, '');
  if (patchedGitignore === gitignore) {
    throw new Error(`vinext patch: \`__lint_rule_fixtures__-*\` not found in ${gitignorePath}`);
  }
  await writeFile(gitignorePath, patchedGitignore, 'utf-8');
}

if (project === 'dify') {
  // pnpm 11 defaults `minimumReleaseAge` to 24 hours when the setting is
  // omitted, so keep dify's explicit opt-out instead of deleting it. Switch
  // this ephemeral fixture away from `resolutionMode: time-based` as well:
  // defining `minimumReleaseAge` there activates a resolution-policy path that
  // vp's bundled pnpm cannot handle.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf-8');
  if (!/^minimumReleaseAge:/m.test(workspace)) {
    throw new Error(`dify patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  if (!/^resolutionMode:\s*time-based[ \t]*$/m.test(workspace)) {
    throw new Error(`dify patch: \`resolutionMode: time-based\` not found in ${workspacePath}`);
  }
  const patched = workspace
    .replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0')
    .replace(/^resolutionMode:\s*time-based[ \t]*$/m, 'resolutionMode: highest');
  await writeFile(workspacePath, patched, 'utf-8');
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
  const workspace = await readFile(workspacePath, 'utf-8');
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
  await writeFile(workspacePath, patched, 'utf-8');
}

// Projects that already use vite-plus need VP_FORCE_MIGRATE=1 so
// vp migrate runs full dependency rewriting instead of skipping.
const forceFreshMigration = 'forceFreshMigration' in repoConfig && repoConfig.forceFreshMigration;

// Mirror VITE_PLUS_OVERRIDE_PACKAGES: pin `vitest` only. The `@vitest/*` family
// are exact deps of `vitest`, so a single `vitest` override cascades them.
//
// Coverage providers are intentionally NOT in the shipped override map (the
// product leaves them user-owned; the runtime guard fail-fasts on a skew). But
// this rig FORCE-INSTALLS the locally built vitest, and many ecosystem projects
// pin an older `@vitest/coverage-*` in their lockfile. Without alignment, the
// forced runner (4.1.9) skews from the project's pinned provider and the guard
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
  /^resolutionMode:\s*time-based/m.test(readFileSync(workspaceYamlPath, 'utf-8'));
const releaseAgeEnv = timeBasedResolution
  ? {}
  : {
      pnpm_config_minimum_release_age: '0',
      PNPM_CONFIG_MINIMUM_RELEASE_AGE: '0',
    };

const migrateEnv: NodeJS.ProcessEnv = {
  ...process.env,
  ...registryInfo.env,
  ...(forceFreshMigration ? { VP_FORCE_MIGRATE: '1' } : {}),
  VP_OVERRIDE_PACKAGES: JSON.stringify({
    vite: `npm:@voidzero-dev/vite-plus-core@${vitePlusVersion}`,
    ...vitestOverrides,
  }),
  // The vp binary was built before the pack step pinned the package versions,
  // so align the version migrate pins with the tgz the registry serves.
  VP_VERSION: vitePlusVersion,
  ...releaseAgeEnv,
};

execSync(`${cli} migrate --no-agent --no-interactive`, {
  cwd: repoRoot,
  stdio: 'inherit',
  env: migrateEnv,
});

if (project === 'tiptap') {
  // Keep Tiptap's upstream lint semantics. Migration enables type-aware type
  // checking, which reports TypeScript diagnostics that upstream CI does not check.
  const viteConfigPath = join(repoRoot, 'vite.config.mts');
  const viteConfig = await readFile(viteConfigPath, 'utf-8');
  const typeAwareOptions =
    /,\s*(?:"options"|options):\s*\{\s*(?:"typeAware"|typeAware):\s*true,\s*(?:"typeCheck"|typeCheck):\s*true\s*\}/;
  const patched = viteConfig.replace(typeAwareOptions, '');
  if (patched === viteConfig) {
    throw new Error(
      `tiptap patch: migrated type-aware lint options not found in ${viteConfigPath}`,
    );
  }
  await writeFile(viteConfigPath, patched, 'utf-8');
}

// Install through the local registry. `vp migrate` already pinned
// `vite-plus@<version>` in package.json exactly like a real migration, so no
// manual package.json rewrite is needed.
execSync(`${cli} install --no-frozen-lockfile`, {
  cwd: repoRoot,
  stdio: 'inherit',
  env: { ...process.env, ...registryInfo.env, ...releaseAgeEnv },
});
