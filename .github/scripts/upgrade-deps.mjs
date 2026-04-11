import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();

// ============ GitHub API ============
async function getLatestTagCommit(owner, repo) {
  const res = await fetch(`https://api.github.com/repos/${owner}/${repo}/tags`, {
    headers: {
      Authorization: `token ${process.env.GITHUB_TOKEN}`,
      Accept: 'application/vnd.github.v3+json',
    },
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch tags for ${owner}/${repo}: ${res.status} ${res.statusText}`);
  }
  const tags = await res.json();
  if (!Array.isArray(tags) || !tags.length) {
    throw new Error(`No tags found for ${owner}/${repo}`);
  }
  if (!tags[0]?.commit?.sha) {
    throw new Error(`Invalid tag structure for ${owner}/${repo}: missing commit SHA`);
  }
  console.log(`${repo} -> ${tags[0].name}`);
  return tags[0].commit.sha;
}

// ============ npm Registry ============
async function getLatestNpmVersion(packageName) {
  const res = await fetch(`https://registry.npmjs.org/${packageName}/latest`);
  if (!res.ok) {
    throw new Error(
      `Failed to fetch npm version for ${packageName}: ${res.status} ${res.statusText}`,
    );
  }
  const data = await res.json();
  if (!data?.version) {
    throw new Error(`Invalid npm response for ${packageName}: missing version field`);
  }
  return data.version;
}

// ============ Update .upstream-versions.json ============
async function updateUpstreamVersions() {
  const filePath = path.join(ROOT, 'packages/tools/.upstream-versions.json');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // rolldown -> rolldown/rolldown
  data.rolldown.hash = await getLatestTagCommit('rolldown', 'rolldown');

  // vite -> vitejs/vite
  data['vite'].hash = await getLatestTagCommit('vitejs', 'vite');

  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
  console.log('Updated .upstream-versions.json');
}

// ============ Update pnpm-workspace.yaml ============
async function updatePnpmWorkspace(versions) {
  const filePath = path.join(ROOT, 'pnpm-workspace.yaml');
  let content = fs.readFileSync(filePath, 'utf8');

  // Update vitest-dev override (handle pre-release versions like -beta.1, -rc.0)
  // Handle both quoted ('npm:vitest@^...') and unquoted (npm:vitest@^...) forms
  content = content.replace(
    /vitest-dev: '?npm:vitest@\^[\d.]+(-[\w.]+)?'?/,
    `vitest-dev: 'npm:vitest@^${versions.vitest}'`,
  );

  // Update tsdown in catalog (handle pre-release versions)
  content = content.replace(/tsdown: \^[\d.]+(-[\w.]+)?/, `tsdown: ^${versions.tsdown}`);

  // Update @oxc-node/cli in catalog
  content = content.replace(
    /'@oxc-node\/cli': \^[\d.]+(-[\w.]+)?/,
    `'@oxc-node/cli': ^${versions.oxcNodeCli}`,
  );

  // Update @oxc-node/core in catalog
  content = content.replace(
    /'@oxc-node\/core': \^[\d.]+(-[\w.]+)?/,
    `'@oxc-node/core': ^${versions.oxcNodeCore}`,
  );

  // Update oxfmt in catalog
  content = content.replace(/oxfmt: =[\d.]+(-[\w.]+)?/, `oxfmt: =${versions.oxfmt}`);

  // Update oxlint in catalog (but not oxlint-tsgolint)
  content = content.replace(/oxlint: =[\d.]+(-[\w.]+)?\n/, `oxlint: =${versions.oxlint}\n`);

  // Update oxlint-tsgolint in catalog
  content = content.replace(
    /oxlint-tsgolint: =[\d.]+(-[\w.]+)?/,
    `oxlint-tsgolint: =${versions.oxlintTsgolint}`,
  );

  fs.writeFileSync(filePath, content);
  console.log('Updated pnpm-workspace.yaml');
}

// ============ Update packages/test/package.json ============
async function updateTestPackage(vitestVersion) {
  const filePath = path.join(ROOT, 'packages/test/package.json');
  const pkg = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // Update all @vitest/* devDependencies
  for (const dep of Object.keys(pkg.devDependencies)) {
    if (dep.startsWith('@vitest/')) {
      pkg.devDependencies[dep] = vitestVersion;
    }
  }

  // Update vitest-dev devDependency
  if (pkg.devDependencies['vitest-dev']) {
    pkg.devDependencies['vitest-dev'] = `^${vitestVersion}`;
  }

  // Update @vitest/ui peerDependency if present
  if (pkg.peerDependencies?.['@vitest/ui']) {
    pkg.peerDependencies['@vitest/ui'] = vitestVersion;
  }

  fs.writeFileSync(filePath, JSON.stringify(pkg, null, 2) + '\n');
  console.log('Updated packages/test/package.json');
}

// ============ Update packages/core/package.json ============
async function updateCorePackage(devtoolsVersion) {
  const filePath = path.join(ROOT, 'packages/core/package.json');
  const pkg = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // Update @vitejs/devtools in devDependencies
  if (pkg.devDependencies?.['@vitejs/devtools']) {
    pkg.devDependencies['@vitejs/devtools'] = `^${devtoolsVersion}`;
  }

  fs.writeFileSync(filePath, JSON.stringify(pkg, null, 2) + '\n');
  console.log('Updated packages/core/package.json');
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
] = await Promise.all([
  getLatestNpmVersion('vitest'),
  getLatestNpmVersion('tsdown'),
  getLatestNpmVersion('@vitejs/devtools'),
  getLatestNpmVersion('@oxc-node/cli'),
  getLatestNpmVersion('@oxc-node/core'),
  getLatestNpmVersion('oxfmt'),
  getLatestNpmVersion('oxlint'),
  getLatestNpmVersion('oxlint-tsgolint'),
]);

console.log(`vitest: ${vitestVersion}`);
console.log(`tsdown: ${tsdownVersion}`);
console.log(`@vitejs/devtools: ${devtoolsVersion}`);
console.log(`@oxc-node/cli: ${oxcNodeCliVersion}`);
console.log(`@oxc-node/core: ${oxcNodeCoreVersion}`);
console.log(`oxfmt: ${oxfmtVersion}`);
console.log(`oxlint: ${oxlintVersion}`);
console.log(`oxlint-tsgolint: ${oxlintTsgolintVersion}`);

await updateUpstreamVersions();
await updatePnpmWorkspace({
  vitest: vitestVersion,
  tsdown: tsdownVersion,
  oxcNodeCli: oxcNodeCliVersion,
  oxcNodeCore: oxcNodeCoreVersion,
  oxfmt: oxfmtVersion,
  oxlint: oxlintVersion,
  oxlintTsgolint: oxlintTsgolintVersion,
});
await updateTestPackage(vitestVersion);
await updateCorePackage(devtoolsVersion);

console.log('Done!');
