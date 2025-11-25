import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format, formatEmbeddedCode } from 'oxfmt';
import {
  createCompilerHost,
  createProgram,
  formatDiagnostics,
  parseJsonSourceFileConfigFileContent,
  readJsonConfigFile,
  sys,
} from 'typescript';

const projectDir = dirname(fileURLToPath(import.meta.url));

await buildCli();
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
  await format(
    outputs.filter((o) => o.kind !== 'node').map((o) => o.path),
    formatEmbeddedCode,
  );
}

async function buildCli() {
  const tsconfig = readJsonConfigFile(
    join(projectDir, 'tsconfig.json'),
    sys.readFile,
  );

  const { options, fileNames } = parseJsonSourceFileConfigFileContent(
    tsconfig,
    sys,
    projectDir,
  );

  const host = createCompilerHost(options);

  const program = createProgram({
    rootNames: fileNames,
    options,
    host,
  });

  const { diagnostics } = program.emit();

  if (diagnostics.length > 0) {
    console.error(formatDiagnostics(diagnostics, host));
    process.exit(1);
  }
}
