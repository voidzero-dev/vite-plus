import type { CANCEL_SYMBOL } from '@clack/core';
import { ConfirmPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  promptTitle,
  S_BAR,
  S_BAR_END,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
} from './common.js';
import { promptMilestone } from './milestone.js';

export interface ConfirmOptions extends CommonOptions {
  message: string;
  active?: string;
  inactive?: string;
  initialValue?: boolean;
  vertical?: boolean;
}
export const confirm = (opts: ConfirmOptions) => {
  const active = opts.active ?? 'Yes';
  const inactive = opts.inactive ?? 'No';
  return new ConfirmPrompt({
    active,
    inactive,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    output: opts.output,
    initialValue: opts.initialValue ?? true,
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);
      const value = this.value ? active : inactive;

      switch (this.state) {
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${title}${submitPrefix}${color.dim(value)}\n${promptMilestone('confirm', opts.testId, 'submit')}`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${title}${cancelPrefix}${color.strikethrough(
            color.dim(value),
          )}${hasGuide ? `\n${color.gray(S_BAR)}` : ''}\n${promptMilestone('confirm', opts.testId, 'cancel')}`;
        }
        default: {
          const defaultPrefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const defaultPrefixEnd = hasGuide ? color.blue(S_BAR_END) : '';
          return `${title}${defaultPrefix}${
            this.value
              ? `${color.blue(S_POINTER_ACTIVE)} ${color.bold(active)}`
              : `${color.dim(S_POINTER_INACTIVE)} ${color.dim(active)}`
          }${
            opts.vertical
              ? hasGuide
                ? `\n${color.blue(S_BAR)} `
                : `\n${nestedPrefix}`
              : ` ${color.dim('/')} `
          }${
            !this.value
              ? `${color.blue(S_POINTER_ACTIVE)} ${color.bold(inactive)}`
              : `${color.dim(S_POINTER_INACTIVE)} ${color.dim(inactive)}`
          }${hasGuide ? `\n${defaultPrefixEnd}` : ''}\n${promptMilestone('confirm', opts.testId, this.value ? 'yes' : 'no')}`;
        }
      }
    },
  }).prompt() as Promise<boolean | typeof CANCEL_SYMBOL>;
};
