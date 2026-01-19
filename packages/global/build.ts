import { cpSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, parse } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format } from 'oxfmt';

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
  const viteConfig = await import('../../vite.config');

  for (const output of outputs) {
    if (output.kind !== 'node') {
      const { code, errors } = await format(output.path, readFileSync(output.path, 'utf8'), {
        ...viteConfig.default.fmt,
        embeddedCode: true,
      });
      if (errors.length > 0) {
        for (const error of errors) {
          console.error(error);
        }
        process.exit(1);
      }
      writeFileSync(output.path, code);
    }
  }

  const nodeFile = outputs.find((o) => o.kind === 'node');
  if (nodeFile) {
    cpSync(nodeFile.path, join(projectDir, `dist/${parse(nodeFile.path).base}`));
  }
}
