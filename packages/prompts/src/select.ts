import { SelectPrompt, wrapTextWithPrefix } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  S_BAR,
  S_BAR_END,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
  symbol,
  symbolBar,
} from './common.js';
import { limitOptions } from './limit-options.js';

type Primitive = Readonly<string | boolean | number>;

export type Option<Value> = Value extends Primitive
  ? {
      /**
       * Internal data for this option.
       */
      value: Value;
      /**
       * The optional, user-facing text for this option.
       *
       * By default, the `value` is converted to a string.
       */
      label?: string;
      /**
       * An optional hint to display to the user when
       * this option might be selected.
       *
       * By default, no `hint` is displayed.
       */
      hint?: string;
      /**
       * Whether this option is disabled.
       * Disabled options are visible but cannot be selected.
       *
       * By default, options are not disabled.
       */
      disabled?: boolean;
    }
  : {
      /**
       * Internal data for this option.
       */
      value: Value;
      /**
       * Required. The user-facing text for this option.
       */
      label: string;
      /**
       * An optional hint to display to the user when
       * this option might be selected.
       *
       * By default, no `hint` is displayed.
       */
      hint?: string;
      /**
       * Whether this option is disabled.
       * Disabled options are visible but cannot be selected.
       *
       * By default, options are not disabled.
       */
      disabled?: boolean;
    };

export interface SelectOptions<Value> extends CommonOptions {
  message: string;
  options: Option<Value>[];
  initialValue?: Value;
  maxItems?: number;
}

const computeLabel = (label: string, format: (text: string) => string) => {
  if (!label.includes('\n')) {
    return format(label);
  }
  return label
    .split('\n')
    .map((line) => format(line))
    .join('\n');
};

const withMarker = (
  marker: string,
  label: string,
  format: (text: string) => string,
  firstLineSuffix = '',
) => {
  const lines = label.split('\n');
  if (lines.length === 1) {
    return `${marker} ${format(lines[0])}${firstLineSuffix}`;
  }
  const [firstLine, ...rest] = lines;
  return [
    `${marker} ${format(firstLine)}${firstLineSuffix}`,
    ...rest.map((line) => `${S_POINTER_INACTIVE} ${format(line)}`),
  ].join('\n');
};

export const select = <Value>(opts: SelectOptions<Value>) => {
  const opt = (
    option: Option<Value>,
    state: 'inactive' | 'active' | 'selected' | 'cancelled' | 'disabled',
  ) => {
    const label = option.label ?? String(option.value);
    const hint = option.hint ? `: ${color.gray(option.hint)}` : '';
    switch (state) {
      case 'disabled':
        return withMarker(
          color.gray(S_POINTER_INACTIVE),
          label,
          (text) => color.strikethrough(color.gray(text)),
          option.hint ? `: ${color.gray(option.hint ?? 'disabled')}` : '',
        );
      case 'selected':
        return computeLabel(label, color.dim);
      case 'active':
        return withMarker(
          color.blue(S_POINTER_ACTIVE),
          label,
          (text) => color.blue(color.bold(text)),
          hint,
        );
      case 'cancelled':
        return computeLabel(label, (str) => color.strikethrough(color.dim(str)));
      default:
        return withMarker(color.dim(S_POINTER_INACTIVE), label, (text) => text, hint);
    }
  };

  return new SelectPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    output: opts.output,
    initialValue: opts.initialValue,
    render() {
      const hasGuide = opts.withGuide ?? false;
      const nestedPrefix = '  ';
      const formatMessageLines = (message: string) => {
        const lines = message.split('\n');
        return lines
          .map((line, index) => `${index === 0 ? `${symbol(this.state)} ` : nestedPrefix}${line}`)
          .join('\n');
      };
      const hasMessage = opts.message.trim().length > 0;
      const messageLines = !hasMessage
        ? ''
        : hasGuide
          ? wrapTextWithPrefix(
              opts.output,
              opts.message,
              `${symbolBar(this.state)} `,
              `${symbol(this.state)} `,
            )
          : formatMessageLines(opts.message);
      const title = hasMessage
        ? `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${messageLines}\n`
        : '';

      switch (this.state) {
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedLines = wrapTextWithPrefix(
            opts.output,
            opt(this.options[this.cursor], 'selected'),
            submitPrefix,
          );
          return `${title}${wrappedLines}\n`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedLines = wrapTextWithPrefix(
            opts.output,
            opt(this.options[this.cursor], 'cancelled'),
            cancelPrefix,
          );
          return `${title}${wrappedLines}${hasGuide ? `\n${color.gray(S_BAR)}` : ''}\n`;
        }
        default: {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const prefixEnd = hasGuide ? color.blue(S_BAR_END) : '';
          // Calculate rowPadding: title lines + footer lines (S_BAR_END + trailing newline)
          const titleLineCount = title ? title.split('\n').length : 0;
          const footerLineCount = hasGuide ? 2 : 1; // S_BAR_END + trailing newline (or just trailing newline)
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            cursor: this.cursor,
            options: this.options,
            maxItems: opts.maxItems,
            columnPadding: prefix.length,
            rowPadding: titleLineCount + footerLineCount,
            style: (item, active) =>
              opt(item, item.disabled ? 'disabled' : active ? 'active' : 'inactive'),
          }).join(`\n${prefix}`)}\n${prefixEnd}\n`;
        }
      }
    },
  }).prompt() as Promise<Value | symbol>;
};
