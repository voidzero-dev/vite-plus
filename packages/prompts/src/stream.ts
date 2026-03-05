import { stripVTControlCharacters as strip } from 'node:util';

import color from 'picocolors';

import { S_ERROR, S_INFO, S_STEP_SUBMIT, S_SUCCESS, S_WARN, completeColor } from './common.js';
import type { LogMessageOptions } from './log.js';

const prefix = '   ';

// TODO (43081j): this currently doesn't support custom `output` writables
// because we rely on `columns` existing (i.e. `process.stdout.columns).
//
// If we want to support `output` being passed in, we will need to use
// a condition like `if (output instance Writable)` to check if it has columns
export const stream = {
  message: async (
    iterable: Iterable<string> | AsyncIterable<string>,
    { symbol = '' }: LogMessageOptions = {},
  ) => {
    process.stdout.write(symbol ? `${symbol}  ` : '');
    const initialWidth = symbol ? 3 : 0;
    let lineWidth = initialWidth;
    for await (let chunk of iterable) {
      chunk = chunk.replace(/\n/g, `\n${prefix}`);
      if (chunk.includes('\n')) {
        lineWidth = initialWidth + strip(chunk.slice(chunk.lastIndexOf('\n'))).length;
      }
      const chunkLen = strip(chunk).length;
      if (lineWidth + chunkLen < process.stdout.columns) {
        lineWidth += chunkLen;
        process.stdout.write(chunk);
      } else {
        process.stdout.write(`\n${prefix}${chunk.trimStart()}`);
        lineWidth = initialWidth + strip(chunk.trimStart()).length;
      }
    }
    process.stdout.write('\n');
  },
  info: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.message(iterable, { symbol: color.blue(S_INFO) });
  },
  success: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.message(iterable, { symbol: completeColor(S_SUCCESS) });
  },
  step: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.message(iterable, { symbol: completeColor(S_STEP_SUBMIT) });
  },
  warn: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.message(iterable, { symbol: color.yellow(S_WARN) });
  },
  /** alias for `log.warn()`. */
  warning: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.warn(iterable);
  },
  error: (iterable: Iterable<string> | AsyncIterable<string>) => {
    return stream.message(iterable, { symbol: color.red(S_ERROR) });
  },
};
