import type { Writable } from 'node:stream';

import color from 'picocolors';

import { type CommonOptions } from './common.js';

export const cancel = (message = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  output.write(`${color.red(message)}\n\n`);
};

export const intro = (title = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  output.write(`${title}\n\n`);
};

export const outro = (message = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  output.write(`${message}\n\n`);
};
