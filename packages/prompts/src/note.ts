import color from 'picocolors';

import { box } from './box.js';
import { type CommonOptions } from './common.js';

export interface NoteOptions extends CommonOptions {
  /** Format note content; readable, un-dimmed text by default. */
  format?: (line: string) => string;
}

/** A compact, enclosed note. Guide bars never replace its side borders. */
export const note = (message = '', title = '', opts?: NoteOptions) => {
  const content = opts?.format ? message.split('\n').map(opts.format).join('\n') : message;
  box(`\n${content}\n`, title, { ...opts, width: 'auto', rounded: true, formatBorder: color.gray });
};
