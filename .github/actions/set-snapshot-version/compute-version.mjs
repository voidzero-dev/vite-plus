import { execSync } from 'node:child_process';

import semver from 'semver';

const npmTag = process.argv[2] || 'latest';

// Get all version tags
const tagsOutput = execSync('git tag -l "v*"', { encoding: 'utf-8' }).trim();
const tags = tagsOutput ? tagsOutput.split('\n') : [];

// Parse and filter to valid semver, then find latest stable (no prerelease)
const stableTags = tags
  .map((tag) => semver.parse(tag.replace(/^v/, '')))
  .filter((v) => v !== null && v.prerelease.length === 0);

let nextVersion;
if (stableTags.length === 0) {
  nextVersion = '0.1.0';
} else {
  stableTags.sort(semver.rcompare);
  const latest = stableTags[0];
  nextVersion = semver.inc(latest, 'patch');
}

let version;
if (npmTag === 'alpha') {
  // Find existing alpha tags for this version
  const alphaPrefix = `v${nextVersion}-alpha.`;
  const alphaTags = tags
    .filter((tag) => tag.startsWith(alphaPrefix))
    .map((tag) => semver.parse(tag.replace(/^v/, '')))
    .filter((v) => v !== null);

  let alphaNum = 0;
  if (alphaTags.length > 0) {
    alphaTags.sort(semver.rcompare);
    alphaNum = alphaTags[0].prerelease[1] + 1;
  }
  version = `${nextVersion}-alpha.${alphaNum}`;
} else {
  version = nextVersion;
}

const latestStable = stableTags.length > 0 ? `v${stableTags[0].version}` : 'none';
console.log(`Computed version: ${version} (latest stable tag: ${latestStable})`);
console.log(`version=${version}`);
