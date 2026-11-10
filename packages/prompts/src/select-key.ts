import type { CANCEL_SYMBOL } from '@clack/core';
import { SelectKeyPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  promptTitle,
  optionText,
  wrapTextWithPrefix,
  S_BAR,
  S_BAR_END,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
} from './common.js';
import type { Option } from './select.js';

export interface SelectKeyOptions<Value extends string> extends CommonOptions {
  message: string;
  options: Option<Value>[];
  initialValue?: Value;
  caseSensitive?: boolean;
}

export const selectKey = <Value extends string>(opts: SelectKeyOptions<Value>) => {
  const withMarker = (marker: string, value: string) =>
    optionText(opts.output, `${marker} `, value, (text) => text);

  const opt = (
    option: Option<Value> | undefined,
    state: 'inactive' | 'active' | 'selected' | 'cancelled' = 'inactive',
  ) => {
    if (!option) {
      return '';
    }
    const label = option.label ?? option.value;
    if (state === 'selected') {
      return color.dim(label);
    }
    if (state === 'cancelled') {
      return color.strikethrough(color.dim(label));
    }
    if (state === 'active') {
      return withMarker(
        color.blue(S_POINTER_ACTIVE),
        `${color.blue(`[${option.value}]`)} ${color.blue(color.bold(label))}${
          option.hint ? ` ${color.dim(`(${option.hint})`)}` : ''
        }`,
      );
    }
    return withMarker(color.dim(S_POINTER_INACTIVE), color.dim(`[${option.value}] ${label}`));
  };

  return new SelectKeyPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    output: opts.output,
    initialValue: opts.initialValue,
    caseSensitive: opts.caseSensitive,
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);

      switch (this.state) {
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const selectedOption =
            this.options.find((opt) => opt.value === this.value) ?? opts.options[0];
          const wrapped = wrapTextWithPrefix(
            opts.output,
            opt(selectedOption, 'selected'),
            submitPrefix,
          );
          return `${title}${wrapped}\n`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrapped = wrapTextWithPrefix(
            opts.output,
            opt(this.options[0], 'cancelled'),
            cancelPrefix,
          );
          return `${title}${wrapped}${hasGuide ? `\n${color.gray(S_BAR)}` : ''}\n`;
        }
        default: {
          const defaultPrefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const defaultPrefixEnd = hasGuide ? color.blue(S_BAR_END) : '';
          const wrapped = this.options
            .map((option, i) =>
              wrapTextWithPrefix(
                opts.output,
                opt(option, i === this.cursor ? 'active' : 'inactive'),
                defaultPrefix,
              ),
            )
            .join('\n');
          return `${title}${wrapped}${hasGuide ? `\n${defaultPrefixEnd}` : ''}\n`;
        }
      }
    },
  }).prompt() as Promise<Value | typeof CANCEL_SYMBOL>;
};
