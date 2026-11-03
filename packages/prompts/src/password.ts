import type { CANCEL_SYMBOL, Validate } from '@clack/core';
import { PasswordPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  promptTitle,
  wrapTextWithPrefix,
  S_BAR,
  S_BAR_END,
  S_PASSWORD_MASK,
} from './common.js';

export interface PasswordOptions extends CommonOptions {
  message: string;
  mask?: string;
  /** Hint displayed only while the password is empty. */
  placeholder?: string;
  validate?: Validate<string>;
  clearOnError?: boolean;
}
export const password = (opts: PasswordOptions) => {
  return new PasswordPrompt({
    validate: opts.validate,
    mask: opts.mask ?? S_PASSWORD_MASK,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    output: opts.output,
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);
      const userInput =
        !this.userInput && opts.placeholder
          ? color.dim(opts.placeholder)
          : this.userInputWithCursor;
      const masked = this.masked;

      switch (this.state) {
        case 'validating': {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          return `${title}${prefix}${color.dim(masked ?? '')}\n${prefix}${color.dim('Validating…')}\n`;
        }
        case 'error': {
          const errorPrefix = hasGuide ? `${color.yellow(S_BAR)} ` : nestedPrefix;
          const errorPrefixEnd = hasGuide ? `${color.yellow(S_BAR_END)} ` : '';
          const maskedText = masked ?? '';
          if (opts.clearOnError) {
            this.clear();
          }
          return `${title.trim()}\n${errorPrefix}${maskedText}\n${wrapTextWithPrefix(opts.output, color.yellow(this.error), hasGuide ? errorPrefixEnd : nestedPrefix)}\n`;
        }
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const maskedText = masked ? color.dim(masked) : '';
          return `${title}${submitPrefix}${maskedText}\n`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const maskedText = masked ? color.strikethrough(color.dim(masked)) : '';
          return `${title}${cancelPrefix}${maskedText}${
            masked && hasGuide ? `\n${color.gray(S_BAR)}` : ''
          }\n`;
        }
        default: {
          const defaultPrefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const defaultPrefixEnd = hasGuide ? color.blue(S_BAR_END) : '';
          return `${title}${defaultPrefix}${userInput}${hasGuide ? `\n${defaultPrefixEnd}` : ''}\n`;
        }
      }
    },
  }).prompt() as Promise<string | typeof CANCEL_SYMBOL>;
};
