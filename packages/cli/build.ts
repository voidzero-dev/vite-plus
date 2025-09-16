import { copyFile } from 'node:fs/promises';
import { parse } from 'node:path';
import { parseArgs } from 'node:util';

import { NapiCli } from '@napi-rs/cli';
import { build } from 'rolldown';

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
