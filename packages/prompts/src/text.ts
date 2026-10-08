import type { CANCEL_SYMBOL, Validate } from '@clack/core';
import { TextPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  promptTitle,
  wrapTextWithPrefix,
  S_BAR,
  S_BAR_END,
} from './common.js';
import { promptMilestone } from './milestone.js';

export interface TextOptions extends CommonOptions {
  message: string;
  placeholder?: string;
  defaultValue?: string;
  initialValue?: string;
  validate?: Validate<string>;
}

export const text = (opts: TextOptions) => {
  return new TextPrompt({
    validate: opts.validate,
    placeholder: opts.placeholder,
    defaultValue: opts.defaultValue,
    initialValue: opts.initialValue,
    output: opts.output,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);
      const placeholder = opts.placeholder
        ? color.inverse(opts.placeholder[0]) + color.dim(opts.placeholder.slice(1))
        : color.inverse(color.hidden('_'));
      const userInput = !this.userInput ? placeholder : this.userInputWithCursor;
      const value = this.value ?? '';

      switch (this.state) {
        case 'validating': {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          return `${title}${prefix}${color.dim(value)}\n${prefix}${color.dim('Validating…')}\n`;
        }
        case 'error': {
          const errorText = this.error
            ? wrapTextWithPrefix(
                opts.output,
                color.yellow(this.error),
                hasGuide ? `${color.yellow(S_BAR_END)} ` : nestedPrefix,
              )
            : '';
          const errorPrefix = hasGuide ? `${color.yellow(S_BAR)} ` : nestedPrefix;
          return `${title.trim()}\n${errorPrefix}${userInput}\n${errorText}\n${promptMilestone('text', opts.testId, 'error')}`;
        }
        case 'submit': {
          const valueText = value ? color.dim(value) : '';
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${title}${submitPrefix}${valueText}\n${promptMilestone('text', opts.testId, 'submit')}`;
        }
        case 'cancel': {
          const valueText = value ? color.strikethrough(color.dim(value)) : '';
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${title}${cancelPrefix}${valueText}${value.trim() ? `\n${cancelPrefix}` : ''}\n${promptMilestone('text', opts.testId, 'cancel')}`;
        }
        default: {
          const defaultPrefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const defaultPrefixEnd = hasGuide ? color.blue(S_BAR_END) : '';
          return `${title}${defaultPrefix}${userInput}${hasGuide ? `\n${defaultPrefixEnd}` : ''}\n${promptMilestone('text', opts.testId, this.value ?? '')}`;
        }
      }
    },
  }).prompt() as Promise<string | typeof CANCEL_SYMBOL>;
};
