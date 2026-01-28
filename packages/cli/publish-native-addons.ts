import { execSync } from 'node:child_process';
import { readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { NapiCli } from '@napi-rs/cli';

const cli = new NapiCli();

const currentDir = dirname(fileURLToPath(import.meta.url));

await cli.createNpmDirs({
  cwd: currentDir,
  packageJsonPath: './package.json',
});

await cli.artifacts({
  cwd: currentDir,
  packageJsonPath: './package.json',
});

await cli.prePublish({
  cwd: currentDir,
  packageJsonPath: './package.json',
  tagStyle: 'npm',
  ghRelease: false,
  skipOptionalPublish: true,
});

const npmTag = process.env.NPM_TAG || 'latest';
const npmDir = await readdir(join(currentDir, 'npm'));
for (const file of npmDir) {
  execSync(`npm publish --tag ${npmTag} --access public --no-git-checks`, {
    cwd: join(currentDir, 'npm', file),
    env: process.env,
    stdio: 'inherit',
  });
}
