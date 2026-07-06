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

type LatestTagOptions = {
  stableOnly?: boolean;
};

type NpmLatestResponse = {
  version?: unknown;
  dependencies?: Record<string, string>;
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
  lightningcss: string;
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

const STABLE_SEMVER_TAG_RE = /^v?\d+\.\d+\.\d+$/;

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
async function getLatestTag(
  owner: string,
  repo: string,
  options: LatestTagOptions = {},
): Promise<LatestTag> {
  const perPage = options.stableOnly ? 100 : 1;
  const res = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/tags?per_page=${perPage}`,
    {
      headers: {
        Authorization: `token ${process.env.GITHUB_TOKEN}`,
        Accept: 'application/vnd.github.v3+json',
      },
    },
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch tags for ${owner}/${repo}: ${res.status} ${res.statusText}`);
  }
  const tags = (await res.json()) as GitHubTag[];
  if (!Array.isArray(tags) || !tags.length) {
    throw new Error(`No tags found for ${owner}/${repo}`);
  }
  const latest = options.stableOnly
    ? tags.find((tag) => typeof tag.name === 'string' && STABLE_SEMVER_TAG_RE.test(tag.name))
    : tags[0];
  if (!latest) {
    throw new Error(`No stable semver tags found for ${owner}/${repo}`);
  }
  if (typeof latest?.commit?.sha !== 'string' || typeof latest.name !== 'string') {
    throw new Error(`Invalid tag structure for ${owner}/${repo}: missing SHA or name`);
  }
  console.log(`${repo} -> ${latest.name} (${latest.commit.sha.slice(0, 7)})`);
  return { sha: latest.commit.sha, tag: latest.name };
}

// ============ npm Registry ============
async function fetchNpmLatest(packageName: string): Promise<NpmLatestResponse> {
  const res = await fetch(`https://registry.npmjs.org/${packageName}/latest`);
  if (!res.ok) {
    throw new Error(
      `Failed to fetch npm metadata for ${packageName}: ${res.status} ${res.statusText}`,
    );
  }
  return (await res.json()) as NpmLatestResponse;
}

async function getLatestNpmVersion(packageName: string): Promise<string> {
  const data = await fetchNpmLatest(packageName);
  if (typeof data.version !== 'string') {
    throw new Error(`Invalid npm response for ${packageName}: missing version field`);
  }
  return data.version;
}

// Read a dependency range from the latest published version of `packageName`,
// e.g. the `lightningcss` range that the bundled `@tsdown/css` depends on.
async function getNpmDependencyRange(packageName: string, dependencyName: string): Promise<string> {
  const data = await fetchNpmLatest(packageName);
  const range = data.dependencies?.[dependencyName];
  if (typeof range !== 'string') {
    throw new Error(
      `Invalid npm response for ${packageName}: missing dependencies.${dependencyName}`,
    );
  }
  return range;
}

// ============ Update .upstream-versions.json ============
async function updateUpstreamVersions(): Promise<void> {
  const filePath = path.join(ROOT, 'packages/tools/.upstream-versions.json');
  const data: UpstreamVersions = readJsonFile(filePath);

  const oldRolldownHash = data.rolldown.hash;
  const oldViteHash = data.vite.hash;
  const [rolldown, vite] = await Promise.all([
    getLatestTag('rolldown', 'rolldown'),
    getLatestTag('vitejs', 'vite', { stableOnly: true }),
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
  // All @vitest/* catalog entries (browser + core direct deps) must stay pinned
  // to the same exact version as `vitest` itself, otherwise the catalog drifts
  // from VITEST_VERSION.
  const vitestExactVersionPackages = [
    '@vitest/browser',
    '@vitest/browser-playwright',
    '@vitest/browser-preview',
    '@vitest/browser-webdriverio',
    '@vitest/expect',
    '@vitest/mocker',
    '@vitest/pretty-format',
    '@vitest/runner',
    '@vitest/snapshot',
    '@vitest/spy',
    '@vitest/utils',
  ];
  const vitestExactVersionEntries: PnpmWorkspaceEntry[] = vitestExactVersionPackages.map((pkg) => ({
    name: pkg,
    pattern: new RegExp(`'${pkg.replaceAll('/', '\\/')}': ([\\d.]+(?:-[\\w.]+)?)`),
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
    ...vitestExactVersionEntries,
    {
      name: 'tsdown',
      pattern: /tsdown: \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `tsdown: ^${versions.tsdown}`,
      newVersion: versions.tsdown,
    },
    // `@tsdown/css` and `@tsdown/exe` are bundled into core and published in
    // lockstep with tsdown (they exact-peer-depend on the same tsdown version),
    // so pin both catalog entries to the tsdown version to avoid drift.
    {
      name: '@tsdown/css',
      pattern: /'@tsdown\/css': \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@tsdown/css': ^${versions.tsdown}`,
      newVersion: versions.tsdown,
    },
    {
      name: '@tsdown/exe',
      pattern: /'@tsdown\/exe': \^([\d.]+(?:-[\w.]+)?)/,
      replacement: `'@tsdown/exe': ^${versions.tsdown}`,
      newVersion: versions.tsdown,
    },
    // `lightningcss` is a core dependency consumed by the bundled `@tsdown/css`.
    // Track exactly what `@tsdown/css` requires (already an `^x.y.z` range) so a
    // tsdown upgrade that bumps lightningcss is mirrored here.
    {
      name: 'lightningcss',
      // Match any range value (not just `^x.y.z`) so the pattern can re-match
      // whatever `@tsdown/css` declares (`~`, `>=`, compound ranges) on the next run.
      pattern: /\n {2}lightningcss: ([^\n]+)\n/,
      replacement: `\n  lightningcss: ${versions.lightningcss}\n`,
      newVersion: versions.lightningcss,
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

// ============ Update README.md manual-migration vitest pins ============
// The manual-migration guide pins `vitest` to an exact version in three places —
// the npm/Bun `overrides` block, the pnpm-workspace `overrides` block, and the
// Yarn `resolutions` block — so a hand-migrated project shares one Vitest copy
// with the bundled `vp test`. Those literals are NOT interpolated from
// VITEST_VERSION, so a daily bump must rewrite them or the guide drifts behind the
// bundled version. The packages/cli/README.md mirror (refreshed from the root
// README's suffix at build time) is kept in sync here too so the daily PR stays
// self-consistent without depending on a build step running first.
async function updateReadmeVitestPins(vitestVersion: string): Promise<void> {
  const readmePaths = [path.join(ROOT, 'README.md'), path.join(ROOT, 'packages/cli/README.md')];
  // JSON form: `"vitest": "4.1.9"` — npm/Bun `overrides` + Yarn `resolutions` (2 blocks)
  const jsonPattern = /("vitest": ")[\d.]+(?:-[\w.]+)?(")/g;
  // YAML form: `  vitest: 4.1.9` — pnpm-workspace `overrides` (1 block)
  const yamlPattern = /(\n\s*vitest: )[\d.]+(?:-[\w.]+)?(\n)/g;
  // Both READMEs carry the same three manual-migration pins (the cli copy mirrors the
  // root's suffix). Assert the exact shape so a daily run fails loudly — like every
  // other updater here — if a block is reworded or removed, instead of silently
  // shipping a README where only some pins were bumped.
  const EXPECTED_JSON = 2;
  const EXPECTED_YAML = 1;
  let oldVersion: string | undefined;
  const capture = (match: string): void => {
    const found = /(\d[\d.]*(?:-[\w.]+)?)/.exec(match)?.[1];
    if (found && oldVersion === undefined) {
      oldVersion = found;
    }
  };
  for (const filePath of readmePaths) {
    if (!fs.existsSync(filePath)) {
      continue;
    }
    const content = fs.readFileSync(filePath, 'utf8');
    let jsonMatched = 0;
    let yamlMatched = 0;
    let updated = content.replace(jsonPattern, (match: string, pre: string, post: string) => {
      jsonMatched++;
      capture(match);
      return `${pre}${vitestVersion}${post}`;
    });
    updated = updated.replace(yamlPattern, (match: string, pre: string, post: string) => {
      yamlMatched++;
      capture(match);
      return `${pre}${vitestVersion}${post}`;
    });
    if (jsonMatched !== EXPECTED_JSON || yamlMatched !== EXPECTED_YAML) {
      throw new Error(
        `Expected ${EXPECTED_JSON} JSON + ${EXPECTED_YAML} YAML vitest pins in ${filePath}, ` +
          `found ${jsonMatched} + ${yamlMatched} — the manual-migration section changed, ` +
          `please update updateReadmeVitestPins in .github/scripts/upgrade-deps.ts`,
      );
    }
    fs.writeFileSync(filePath, updated);
    console.log(`Updated ${path.relative(ROOT, filePath)}`);
  }
  recordChange('README vitest pins', oldVersion ?? null, vitestVersion);
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
  lightningcssVersion,
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
  // Mirror exactly what the bundled @tsdown/css depends on.
  getNpmDependencyRange('@tsdown/css', 'lightningcss'),
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
console.log(`lightningcss (from @tsdown/css): ${lightningcssVersion}`);
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
  lightningcss: lightningcssVersion,
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
await updateReadmeVitestPins(vitestVersion);
await updateCorePackage(devtoolsVersion);

writeMetaFiles();

console.log('Done!');
