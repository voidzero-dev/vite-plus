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

  // rolldown-vite -> vitejs/vite
  data['rolldown-vite'].hash = await getLatestTagCommit('vitejs', 'vite');

  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
  console.log('Updated .upstream-versions.json');
}

// ============ Update pnpm-workspace.yaml ============
async function updatePnpmWorkspace(vitestVersion, tsdownVersion) {
  const filePath = path.join(ROOT, 'pnpm-workspace.yaml');
  let content = fs.readFileSync(filePath, 'utf8');

  // Update vitest-dev override (handle pre-release versions like -beta.1, -rc.0)
  content = content.replace(
    /vitest-dev: 'npm:vitest@\^[\d.]+(-[\w.]+)?'/,
    `vitest-dev: 'npm:vitest@^${vitestVersion}'`,
  );

  // Update tsdown in catalog (handle pre-release versions)
  content = content.replace(/tsdown: \^[\d.]+(-[\w.]+)?/, `tsdown: ^${tsdownVersion}`);

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

// ============ Main ============
async function main() {
  console.log('Fetching latest versions...');

  const [vitestVersion, tsdownVersion, devtoolsVersion] = await Promise.all([
    getLatestNpmVersion('vitest'),
    getLatestNpmVersion('tsdown'),
    getLatestNpmVersion('@vitejs/devtools'),
  ]);

  console.log(`vitest: ${vitestVersion}`);
  console.log(`tsdown: ${tsdownVersion}`);
  console.log(`@vitejs/devtools: ${devtoolsVersion}`);

  await updateUpstreamVersions();
  await updatePnpmWorkspace(vitestVersion, tsdownVersion);
  await updateTestPackage(vitestVersion);
  await updateCorePackage(devtoolsVersion);

  console.log('Done!');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
