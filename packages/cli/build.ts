import { copyFile } from 'node:fs/promises';
import { join, parse } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { NapiCli } from '@napi-rs/cli';
import { build } from 'rolldown';
import {
  createCompilerHost,
  createProgram,
  formatDiagnostics,
  parseJsonSourceFileConfigFileContent,
  readJsonConfigFile,
  sys,
} from 'typescript';

const { values: { target, x } } = parseArgs({
  options: {
    target: {
      type: 'string',
    },
    x: {
      type: 'boolean',
      default: false,
    },
  },
});

const cli = new NapiCli();
const { task } = await cli.build({
  packageJsonPath: '../package.json',
  cwd: 'binding',
  platform: true,
  release: true,
  esm: true,
  target,
  crossCompile: x,
});

const output = (await task).find((o) => o.kind === 'node');

await build({
  input: ['./src/bin.ts', './src/index.ts'],
  external: [/^node:/, 'rolldown-vite'],
  output: {
    format: 'esm',
  },
});

if (output) {
  await copyFile(output.path, `./dist/${parse(output.path).base}`);
}

const projectDir = join(fileURLToPath(import.meta.url), '..');

const tsconfig = readJsonConfigFile(join(projectDir, 'tsconfig.json'), sys.readFile);

const { options } = parseJsonSourceFileConfigFileContent(tsconfig, sys, projectDir);

const host = createCompilerHost(options);

const program = createProgram({
  rootNames: [join(projectDir, 'src', 'index.ts')],
  options: {
    ...options,
    emitDeclarationOnly: true,
  },
  host,
});

const { diagnostics } = program.emit();

if (diagnostics.length > 0) {
  console.error(formatDiagnostics(diagnostics, host));
  process.exit(1);
}
