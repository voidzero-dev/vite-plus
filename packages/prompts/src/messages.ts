import type { Writable } from 'node:stream';

import color from 'picocolors';

import { type CommonOptions, getGuide, S_BAR, S_BAR_START, S_BAR_END } from './common.js';

export const cancel = (message = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  const prefix = getGuide(opts) ? `${color.gray(S_BAR_END)} ` : '';
  output.write(`${prefix}${color.red(message)}\n\n`);
};

export const intro = (title = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  const prefix = getGuide(opts) ? `${color.gray(S_BAR_START)} ` : '';
  output.write(`${prefix}${title}\n\n`);
};

export const outro = (message = '', opts?: CommonOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  const prefix = getGuide(opts) ? `${color.gray(S_BAR)}\n${color.gray(S_BAR_END)} ` : '';
  output.write(`${prefix}${message}\n\n`);
};
