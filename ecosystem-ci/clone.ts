import { execSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

import { ecosystemCiDir } from './paths.ts';
import repos from './repo.json' with { type: 'json' };

const cwd = import.meta.dirname;

function exec(cmd: string, execCwd: string = cwd): string {
  return execSync(cmd, { cwd: execCwd, encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }).trim();
}

function getRemoteUrl(dir: string): string | null {
  try {
    return exec('git remote get-url origin', dir);
  } catch {
    return null;
  }
}

function normalizeGitUrl(url: string): string {
  // Convert git@github.com:owner/repo.git to github.com/owner/repo
  // Convert https://github.com/owner/repo.git to github.com/owner/repo
  return url
    .replace(/^git@([^:]+):/, '$1/')
    .replace(/^https?:\/\//, '')
    .replace(/\.git$/, '');
}

function isSameRepo(url1: string, url2: string): boolean {
  return normalizeGitUrl(url1) === normalizeGitUrl(url2);
}

function getCurrentHash(dir: string): string | null {
  try {
    return exec('git rev-parse HEAD', dir);
  } catch {
    return null;
  }
}

function cloneRepo(repoUrl: string, targetDir: string): void {
  console.info(`Cloning ${repoUrl}…`);
  exec(`git clone --depth 1 ${repoUrl} ${targetDir}`);
}

function checkoutHash(dir: string, hash: string): void {
  console.info(`Checking out ${hash.slice(0, 7)}…`);
  exec(`git fetch --depth 1 origin ${hash}`, dir);
  exec(`git checkout ${hash}`, dir);
}

function cloneProject(repoName: string): void {
  const repo = repos[repoName as keyof typeof repos];
  if (!repo) {
    console.error(`Project ${repoName} is not defined in repo.json`);
    process.exit(1);
  }

  const targetDir = join(ecosystemCiDir, repoName);

  if (existsSync(targetDir)) {
    console.info(`Directory ${repoName} exists, validating…`);

    const remoteUrl = getRemoteUrl(targetDir);
    if (!remoteUrl) {
      console.error(`  ✗ ${repoName} is not a git repository`);
      process.exit(1);
    }

    if (!isSameRepo(remoteUrl, repo.repository)) {
      console.error(`  ✗ Remote mismatch: expected ${repo.repository}, got ${remoteUrl}`);
      process.exit(1);
    }

    console.info(`  ✓ Remote matches`);

    const currentHash = getCurrentHash(targetDir);
    if (currentHash === repo.hash) {
      console.info(`  ✓ Already at correct commit ${repo.hash.slice(0, 7)}`);
    } else {
      console.info(`  → Current: ${currentHash?.slice(0, 7)}, expected: ${repo.hash.slice(0, 7)}`);
      checkoutHash(targetDir, repo.hash);
      console.info(`  ✓ Checked out ${repo.hash.slice(0, 7)}`);
    }
  } else {
    cloneRepo(repo.repository, targetDir);
    checkoutHash(targetDir, repo.hash);
    console.info(`✓ Cloned and checked out ${repo.hash.slice(0, 7)}`);
  }
}

// Ensure the directory exists
mkdirSync(ecosystemCiDir, { recursive: true });

const project = process.argv[2];

if (project) {
  // Clone a single project
  console.info(`Cloning ${project} to ${ecosystemCiDir}\n`);
  cloneProject(project);
} else {
  // Clone all projects
  console.info(`Cloning all ecosystem-ci projects to ${ecosystemCiDir}\n`);
  for (const repoName of Object.keys(repos)) {
    cloneProject(repoName);
    console.info('');
  }
}

console.info('Done!');
