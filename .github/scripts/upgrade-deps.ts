import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const META_DIR = process.env.UPGRADE_DEPS_META_DIR;

type Change = {
  old: string | null;
  new: string;
  tag?: string;
};

type GitHubTag = {
  name?: unknown;
  commit?: {
    sha?: unknown;
  };
};

type LatestTag = {
  sha: string;
  tag: string;
};

type NpmLatestResponse = {
  version?: unknown;
};

type UpstreamVersions = {
  rolldown: {
    hash: string;
  };
  vite: {
    hash: string;
  };
};

type PnpmWorkspaceVersions = {
  vitest: string;
  tsdown: string;
  oxcNodeCli: string;
  oxcNodeCore: string;
  oxfmt: string;
  oxlint: string;
  oxlintTsgolint: string;
  oxcProjectRuntime: string;
  oxcProjectTypes: string;
  oxcMinify: string;
  oxcParser: string;
  oxcTransform: string;
};

type PnpmWorkspaceEntry = {
  name: string;
  pattern: RegExp;
  replacement: string;
  newVersion: string;
};

type PackageJson = {
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

const isFullSha = (s: string): boolean => /^[0-9a-f]{40}$/.test(s);

const changes = new Map<string, Change>();

function readJsonFile(filePath: string) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function recordChange(
  name: string,
  oldValue: string | null | undefined,
  newValue: string,
  tag?: string,
) {
  const entry: Change = { old: oldValue ?? null, new: newValue };
  if (tag) {
    entry.tag = tag;
  }
  changes.set(name, entry);
  if (oldValue !== newValue) {
    console.log(`  ${name}: ${oldValue ?? '(unset)'} -> ${newValue}`);
  } else {
    console.log(`  ${name}: ${newValue} (unchanged)`);
  }
}

// ============ GitHub API ============
async function getLatestTag(owner: string, repo: string): Promise<LatestTag> {
  const res = await fetch(`https://api.github.com/repos/${owner}/${repo}/tags?per_page=1`, {
    headers: {
      Authorization: `token ${process.env.GITHUB_TOKEN}`,
      Accept: 'application/vnd.github.v3+json',
    },
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch tags for ${owner}/${repo}: ${res.status} ${res.statusText}`);
  }
  const tags = (await res.json()) as GitHubTag[];
  if (!Array.isArray(tags) || !tags.length) {
    throw new Error(`No tags found for ${owner}/${repo}`);
  }
  const [latest] = tags;
  if (typeof latest?.commit?.sha !== 'string' || typeof latest.name !== 'string') {
    throw new Error(`Invalid tag structure for ${owner}/${repo}: missing SHA or name`);
  }
  console.log(`${repo} -> ${latest.name} (${latest.commit.sha.slice(0, 7)})`);
  return { sha: latest.commit.sha, tag: latest.name };
}

// ============ npm Registry ============
async function getLatestNpmVersion(packageName: string): Promise<string> {
  const res = await fetch(`https://registry.npmjs.org/${packageName}/latest`);
  if (!res.ok) {
    throw new Error(
      `Failed to fetch npm version for ${packageName}: ${res.status} ${res.statusText}`,
    );
  }
  const data = (await res.json()) as NpmLatestResponse;
  if (typeof data.version !== 'string') {
    throw new Error(`Invalid npm response for ${packageName}: missing version field`);
  }
  return data.version;
}

// ============ Update .upstream-versions.json ============
async function updateUpstreamVersions(): Promise<void> {
  const filePath = path.join(ROOT, 'packages/tools/.upstream-versions.json');
  const data: UpstreamVersions = readJsonFile(filePath);

  const oldRolldownHash = data.rolldown.hash;
  const oldViteHash = data.vite.hash;
  const [rolldown, vite] = await Promise.all([
    getLatestTag('rolldown', 'rolldown'),
    getLatestTag('vitejs', 'vite'),
  ]);
  data.rolldown.hash = rolldown.sha;
  data.vite.hash = vite.sha;
  recordChange('rolldown', oldRolldownHash, rolldown.sha, rolldown.tag);
  recordChange('vite', oldViteHash, vite.sha, vite.tag);

  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
  console.log('Updated .upstream-versions.json');
}

// ============ Update pnpm-workspace.yaml ============
async function updatePnpmWorkspace(versions: PnpmWorkspaceVersions): Promise<void> {
  const filePath = path.join(ROOT, 'pnpm-workspace.yaml');
  let content = fs.readFileSync(filePath, 'utf8');

  // oxlint's trailing \n in the pattern disambiguates from oxlint-tsgolint.
  const vitestBrowserPackages = [
    '@vitest/browser',
    '@vitest/browser-playwright',
    '@vitest/browser-preview',
    '@vitest/browser-webdriverio',
  ];
  const vitestBrowserEntries: PnpmWorkspaceEntry[] = vitestBrowserPackages.map((pkg) => ({
    name: pkg,
    pattern: new RegExp(`'${pkg.replace('/', '\\/')}': ([\\d.]+(?:-[\\w.]+)?)`),
    replacement: `'${pkg}': ${versions.vitest}`,
    newVersion: versions.vitest,
  }));
  const entries: PnpmWorkspaceEntry[] = [
    {
      name: 'vitest',
      // The `@voidzero-dev/vite-plus-test` wrapper (which used to be aliased
      // here via `vitest-dev: npm:vitest@^…`) has been removed. Vitest is now
      // a plain catalog entry pinned to an exact version (`vitest: x.y.z`),
      // so match that shape directly. The leading newline anchor disambiguates
      // from neighbouring keys like `vitepress-*` and `@vitest/browser`.
      pattern: /\n {2}vitest: ([\d.]+(?:-[\w.]+)?)\n/,
      replacement: `\n  vitest: ${versions.vitest}\n`,
      newVersion: versions.vitest,
    },
    ...vitestBrowserEntries,
    {
      name: 'tsdown',
      pattern: /tsdown: \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `tsdown: ^${versions.tsdown}`,
      newVersion: versions.tsdown,
    },
    {
      name: '@oxc-node/cli',
      pattern: /'@oxc-node\/cli': \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@oxc-node/cli': ^${versions.oxcNodeCli}`,
      newVersion: versions.oxcNodeCli,
    },
    {
      name: '@oxc-node/core',
      pattern: /'@oxc-node\/core': \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@oxc-node/core': ^${versions.oxcNodeCore}`,
      newVersion: versions.oxcNodeCore,
    },
    {
      name: 'oxfmt',
      pattern: /oxfmt: =([\d.]+(?:-[\w.]+)?)/,
      replacement: `oxfmt: =${versions.oxfmt}`,
      newVersion: versions.oxfmt,
    },
    {
      name: 'oxlint',
      pattern: /oxlint: =([\d.]+(?:-[\w.]+)?)\n/,
      replacement: `oxlint: =${versions.oxlint}\n`,
      newVersion: versions.oxlint,
    },
    {
      name: 'oxlint-tsgolint',
      pattern: /oxlint-tsgolint: =([\d.]+(?:-[\w.]+)?)/,
      replacement: `oxlint-tsgolint: =${versions.oxlintTsgolint}`,
      newVersion: versions.oxlintTsgolint,
    },
    {
      name: '@oxc-project/runtime',
      pattern: /'@oxc-project\/runtime': =([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@oxc-project/runtime': =${versions.oxcProjectRuntime}`,
      newVersion: versions.oxcProjectRuntime,
    },
    {
      name: '@oxc-project/types',
      pattern: /'@oxc-project\/types': =([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@oxc-project/types': =${versions.oxcProjectTypes}`,
      newVersion: versions.oxcProjectTypes,
    },
    {
      name: 'oxc-minify',
      pattern: /oxc-minify: =([\d.]+(?:-[\w.]+)?)/,
      replacement: `oxc-minify: =${versions.oxcMinify}`,
      newVersion: versions.oxcMinify,
    },
    {
      name: 'oxc-parser',
      pattern: /oxc-parser: =([\d.]+(?:-[\w.]+)?)/,
      replacement: `oxc-parser: =${versions.oxcParser}`,
      newVersion: versions.oxcParser,
    },
    {
      name: 'oxc-transform',
      pattern: /oxc-transform: =([\d.]+(?:-[\w.]+)?)/,
      replacement: `oxc-transform: =${versions.oxcTransform}`,
      newVersion: versions.oxcTransform,
    },
  ];

  for (const { name, pattern, replacement, newVersion } of entries) {
    let oldVersion: string | undefined;
    content = content.replace(pattern, (_match: string, captured: string) => {
      oldVersion = captured;
      return replacement;
    });
    if (oldVersion === undefined) {
      throw new Error(
        `Failed to match ${name} in pnpm-workspace.yaml — the pattern ${pattern} is stale, ` +
          `please update it in .github/scripts/upgrade-deps.ts`,
      );
    }
    recordChange(name, oldVersion, newVersion);
  }

  fs.writeFileSync(filePath, content);
  console.log('Updated pnpm-workspace.yaml');
}

// ============ Update VITEST_VERSION constant ============
// Keeps the TypeScript source-of-truth (`packages/cli/src/utils/constants.ts`)
// in sync with the `vitest:` catalog entry in pnpm-workspace.yaml. The
// constant is consumed by both `packages/cli` and `ecosystem-ci/patch-project.ts`
// (which re-imports it), so daily upstream bumps must update it here too.
async function updateVitestVersionConstant(vitestVersion: string): Promise<void> {
  const filePath = path.join(ROOT, 'packages/cli/src/utils/constants.ts');
  const content = fs.readFileSync(filePath, 'utf8');
  const pattern = /export const VITEST_VERSION = '([\d.]+(?:-[\w.]+)?)';/;
  let oldVersion: string | undefined;
  const updated = content.replace(pattern, (_match: string, captured: string) => {
    oldVersion = captured;
    return `export const VITEST_VERSION = '${vitestVersion}';`;
  });
  if (oldVersion === undefined) {
    throw new Error(
      `Failed to match VITEST_VERSION in ${filePath} — the pattern ${pattern} is stale, ` +
        `please update it in .github/scripts/upgrade-deps.ts`,
    );
  }
  fs.writeFileSync(filePath, updated);
  recordChange('VITEST_VERSION constant', oldVersion, vitestVersion);
  console.log('Updated packages/cli/src/utils/constants.ts');
}

// ============ Update .github/workflows/test-vp-create.yml ============
// The `vp create` smoke-test workflow pins every vitest-family package via the
// `VP_OVERRIDE_PACKAGES` env var so that template installs use the bundled
// version. Daily upstream bumps must rewrite those pins so the workflow does
// not drift behind the rest of the repo.
async function updateTestVpCreateWorkflow(vitestVersion: string): Promise<void> {
  const filePath = path.join(ROOT, '.github/workflows/test-vp-create.yml');
  const content = fs.readFileSync(filePath, 'utf8');
  const vitestKeys = [
    'vitest',
    '@vitest/expect',
    '@vitest/runner',
    '@vitest/snapshot',
    '@vitest/spy',
    '@vitest/utils',
    '@vitest/mocker',
    '@vitest/pretty-format',
    '@vitest/coverage-v8',
    '@vitest/coverage-istanbul',
  ];
  let updated = content;
  for (const key of vitestKeys) {
    const pattern = new RegExp(`"${key.replace('/', '\\/')}":"([\\d.]+(?:-[\\w.]+)?)"`);
    let matched = false;
    updated = updated.replace(pattern, (_match: string, _captured: string) => {
      matched = true;
      return `"${key}":"${vitestVersion}"`;
    });
    if (!matched) {
      throw new Error(
        `Failed to match "${key}" in ${filePath} — the pattern ${pattern} is stale, ` +
          `please update it in .github/scripts/upgrade-deps.ts`,
      );
    }
  }
  fs.writeFileSync(filePath, updated);
  console.log('Updated .github/workflows/test-vp-create.yml');
}

// ============ Update packages/core/package.json ============
async function updateCorePackage(devtoolsVersion: string): Promise<void> {
  const filePath = path.join(ROOT, 'packages/core/package.json');
  const pkg: PackageJson = readJsonFile(filePath);

  const devDependencies = pkg.devDependencies;
  const currentDevtools = devDependencies?.['@vitejs/devtools'];
  if (!currentDevtools) {
    return;
  }
  devDependencies['@vitejs/devtools'] = `^${devtoolsVersion}`;
  recordChange('@vitejs/devtools', currentDevtools.replace(/^[\^~]/, ''), devtoolsVersion);

  fs.writeFileSync(filePath, JSON.stringify(pkg, null, 2) + '\n');
  console.log('Updated packages/core/package.json');
}

// ============ Write metadata files for PR description ============
function writeMetaFiles(): void {
  if (!META_DIR) {
    return;
  }

  fs.mkdirSync(META_DIR, { recursive: true });

  const versionsObj = Object.fromEntries(changes);
  fs.writeFileSync(
    path.join(META_DIR, 'versions.json'),
    JSON.stringify(versionsObj, null, 2) + '\n',
  );

  const changed = [...changes.entries()].filter(([, v]) => v.old !== v.new);
  const unchanged = [...changes.entries()].filter(([, v]) => v.old === v.new);

  const formatVersion = (v: Change): string => {
    if (v.tag) {
      return `${v.tag} (${v.new.slice(0, 7)})`;
    }
    if (isFullSha(v.new)) {
      return v.new.slice(0, 7);
    }
    return v.new;
  };
  const formatOld = (v: Change): string => {
    if (!v.old) {
      return '(unset)';
    }
    if (isFullSha(v.old)) {
      return v.old.slice(0, 7);
    }
    return v.old;
  };

  const commitLines = ['feat(deps): upgrade upstream dependencies', ''];
  if (changed.length) {
    for (const [name, v] of changed) {
      commitLines.push(`- ${name}: ${formatOld(v)} -> ${formatVersion(v)}`);
    }
  } else {
    commitLines.push('- no version changes detected');
  }
  commitLines.push('');
  fs.writeFileSync(path.join(META_DIR, 'commit-message.txt'), commitLines.join('\n'));

  const bodyLines = ['## Summary', ''];
  if (changed.length) {
    bodyLines.push('Automated daily upgrade of upstream dependencies.');
  } else {
    bodyLines.push('Automated daily upgrade run — no upstream version changes detected.');
  }
  bodyLines.push('', '## Dependency updates', '');
  if (changed.length) {
    bodyLines.push('| Package | From | To |');
    bodyLines.push('| --- | --- | --- |');
    for (const [name, v] of changed) {
      bodyLines.push(`| \`${name}\` | \`${formatOld(v)}\` | \`${formatVersion(v)}\` |`);
    }
  } else {
    bodyLines.push('_No version changes._');
  }
  if (unchanged.length) {
    bodyLines.push('', '<details><summary>Unchanged dependencies</summary>', '');
    for (const [name, v] of unchanged) {
      bodyLines.push(`- \`${name}\`: \`${formatVersion(v)}\``);
    }
    bodyLines.push('', '</details>');
  }
  bodyLines.push('', '## Code changes', '', '_No additional code changes recorded._', '');
  fs.writeFileSync(path.join(META_DIR, 'pr-body.md'), bodyLines.join('\n'));

  console.log(`Wrote metadata files to ${META_DIR}`);
}

console.log('Fetching latest versions…');

const [
  vitestVersion,
  tsdownVersion,
  devtoolsVersion,
  oxcNodeCliVersion,
  oxcNodeCoreVersion,
  oxfmtVersion,
  oxlintVersion,
  oxlintTsgolintVersion,
  oxcProjectRuntimeVersion,
  oxcProjectTypesVersion,
  oxcMinifyVersion,
  oxcParserVersion,
  oxcTransformVersion,
] = await Promise.all([
  getLatestNpmVersion('vitest'),
  getLatestNpmVersion('tsdown'),
  getLatestNpmVersion('@vitejs/devtools'),
  getLatestNpmVersion('@oxc-node/cli'),
  getLatestNpmVersion('@oxc-node/core'),
  getLatestNpmVersion('oxfmt'),
  getLatestNpmVersion('oxlint'),
  getLatestNpmVersion('oxlint-tsgolint'),
  getLatestNpmVersion('@oxc-project/runtime'),
  getLatestNpmVersion('@oxc-project/types'),
  getLatestNpmVersion('oxc-minify'),
  getLatestNpmVersion('oxc-parser'),
  getLatestNpmVersion('oxc-transform'),
]);

console.log(`vitest: ${vitestVersion}`);
console.log(`tsdown: ${tsdownVersion}`);
console.log(`@vitejs/devtools: ${devtoolsVersion}`);
console.log(`@oxc-node/cli: ${oxcNodeCliVersion}`);
console.log(`@oxc-node/core: ${oxcNodeCoreVersion}`);
console.log(`oxfmt: ${oxfmtVersion}`);
console.log(`oxlint: ${oxlintVersion}`);
console.log(`oxlint-tsgolint: ${oxlintTsgolintVersion}`);
console.log(`@oxc-project/runtime: ${oxcProjectRuntimeVersion}`);
console.log(`@oxc-project/types: ${oxcProjectTypesVersion}`);
console.log(`oxc-minify: ${oxcMinifyVersion}`);
console.log(`oxc-parser: ${oxcParserVersion}`);
console.log(`oxc-transform: ${oxcTransformVersion}`);

await updateUpstreamVersions();
await updatePnpmWorkspace({
  vitest: vitestVersion,
  tsdown: tsdownVersion,
  oxcNodeCli: oxcNodeCliVersion,
  oxcNodeCore: oxcNodeCoreVersion,
  oxfmt: oxfmtVersion,
  oxlint: oxlintVersion,
  oxlintTsgolint: oxlintTsgolintVersion,
  oxcProjectRuntime: oxcProjectRuntimeVersion,
  oxcProjectTypes: oxcProjectTypesVersion,
  oxcMinify: oxcMinifyVersion,
  oxcParser: oxcParserVersion,
  oxcTransform: oxcTransformVersion,
});
await updateVitestVersionConstant(vitestVersion);
await updateTestVpCreateWorkflow(vitestVersion);
await updateCorePackage(devtoolsVersion);

writeMetaFiles();

console.log('Done!');
