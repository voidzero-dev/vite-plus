import { existsSync, mkdirSync, writeFileSync, cpSync } from 'node:fs';
import { dirname, join, parse } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format, formatEmbeddedCode } from 'oxfmt';

const projectDir = dirname(fileURLToPath(import.meta.url));

await buildNapiBinding();

async function buildNapiBinding() {
  const buildCommand = createBuildCommand(process.argv.slice(2));
  const passedInOptions = buildCommand.getOptions();

  const cli = new NapiCli();
  const { task } = await cli.build({
    ...passedInOptions,
    packageJsonPath: '../package.json',
    cwd: 'binding',
    platform: true,
    release: process.env.VITE_PLUS_CLI_DEBUG !== '1',
    esm: true,
  });

  const outputs = await task;
  const fmtConfigPath = join(projectDir, '../../node_modules/.vite/task-cache/.oxfmtrc.json');
  if (!existsSync(fmtConfigPath)) {
    const viteConfig = await import('../../vite.config');
    mkdirSync(dirname(fmtConfigPath), { recursive: true });
    writeFileSync(fmtConfigPath, JSON.stringify(viteConfig.default.fmt, null, 2));
  }
  await format(
    [
      '-c',
      '../../node_modules/.vite/task-cache/.oxfmtrc.json',
      ...outputs.filter((o) => o.kind !== 'node').map((o) => o.path),
    ],
    formatEmbeddedCode,
  );

  const nodeFile = outputs.find((o) => o.kind === 'node');
  if (nodeFile) {
    cpSync(nodeFile.path, join(projectDir, `dist/${parse(nodeFile.path).base}`));
  }
}
