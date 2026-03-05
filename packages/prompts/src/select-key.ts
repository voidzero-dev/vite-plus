import { SelectKeyPrompt, wrapTextWithPrefix } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  S_BAR,
  S_BAR_END,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
  symbol,
} from './common.js';
import type { Option } from './select.js';

export interface SelectKeyOptions<Value extends string> extends CommonOptions {
  message: string;
  options: Option<Value>[];
  initialValue?: Value;
  caseSensitive?: boolean;
}

export const selectKey = <Value extends string>(opts: SelectKeyOptions<Value>) => {
  const withMarker = (marker: string, value: string) => {
    const lines = value.split('\n');
    if (lines.length === 1) {
      return `${marker} ${lines[0]}`;
    }
    const [firstLine, ...rest] = lines;
    return [`${marker} ${firstLine}`, ...rest.map((line) => `${S_POINTER_INACTIVE} ${line}`)].join(
      '\n',
    );
  };

  const opt = (
    option: Option<Value>,
    state: 'inactive' | 'active' | 'selected' | 'cancelled' = 'inactive',
  ) => {
    const label = option.label ?? String(option.value);
    if (state === 'selected') {
      return color.dim(label);
    }
    if (state === 'cancelled') {
      return color.strikethrough(color.dim(label));
    }
    if (state === 'active') {
      return withMarker(
        color.blue(S_POINTER_ACTIVE),
        `${color.bgBlue(color.white(` ${option.value} `))} ${color.bold(label)}${
          option.hint ? ` ${color.dim(`(${option.hint})`)}` : ''
        }`,
      );
    }
    return withMarker(
      color.dim(S_POINTER_INACTIVE),
      `${color.gray(color.bgWhite(color.inverse(` ${option.value} `)))} ${color.dim(label)}${
        option.hint ? ` ${color.dim(`(${option.hint})`)}` : ''
      }`,
    );
  };

  return new SelectKeyPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    output: opts.output,
    initialValue: opts.initialValue,
    caseSensitive: opts.caseSensitive,
    render() {
      const hasGuide = opts.withGuide ?? false;
      const nestedPrefix = '  ';
      const title = `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${symbol(this.state)} ${opts.message}\n`;

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
          return `${title}${wrapped}\n\n`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrapped = wrapTextWithPrefix(
            opts.output,
            opt(this.options[0], 'cancelled'),
            cancelPrefix,
          );
          return `${title}${wrapped}${hasGuide ? `\n${color.gray(S_BAR)}` : ''}\n\n`;
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
          return `${title}${wrapped}\n${defaultPrefixEnd}\n`;
        }
      }
    },
  }).prompt() as Promise<Value | symbol>;
};
