import { cpSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, parse } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format } from 'oxfmt';

const projectDir = dirname(fileURLToPath(import.meta.url));

await buildNapiBinding();
syncReadmeFromRoot();

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

function syncReadmeFromRoot() {
  const rootReadmePath = join(projectDir, '..', '..', 'README.md');
  const packageReadmePath = join(projectDir, 'README.md');
  const rootReadme = readFileSync(rootReadmePath, 'utf8');
  const packageReadme = readFileSync(packageReadmePath, 'utf8');

  const { suffix: rootSuffix } = splitReadme(rootReadme, rootReadmePath);
  const { prefix: packagePrefix } = splitReadme(packageReadme, packageReadmePath);
  const nextReadme = `${packagePrefix}\n\n${rootSuffix}\n`;

  if (nextReadme !== packageReadme) {
    writeFileSync(packageReadmePath, nextReadme);
  }
}

function splitReadme(content: string, label: string) {
  const match = /^---\s*$/m.exec(content);
  if (!match || match.index === undefined) {
    throw new Error(`Expected ${label} to include a '---' separator.`);
  }

  const delimiterStart = match.index;
  const delimiterEnd = delimiterStart + match[0].length;
  const afterDelimiter = content.slice(delimiterEnd);
  const newlineMatch = /^\r?\n/.exec(afterDelimiter);
  const delimiterWithNewlineEnd = delimiterEnd + (newlineMatch ? newlineMatch[0].length : 0);

  return {
    prefix: content.slice(0, delimiterWithNewlineEnd).trim(),
    suffix: content.slice(delimiterWithNewlineEnd).trim(),
  };
}
