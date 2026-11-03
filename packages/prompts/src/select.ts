import type { CANCEL_SYMBOL } from '@clack/core';
import { SelectPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  formatInstructionFooter,
  SELECT_INSTRUCTIONS,
  promptTitle,
  optionText,
  wrapTextWithPrefix,
  S_BAR,
  S_BAR_END,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
} from './common.js';
import { limitOptions } from './limit-options.js';
import { promptMilestone } from './milestone.js';

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
  /** Show keyboard instructions (default: true). */
  showInstructions?: boolean;
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

export const select = <Value>(opts: SelectOptions<Value>) => {
  const withMarker = (
    marker: string,
    label: string,
    format: (text: string) => string,
    suffix = '',
  ) => optionText(opts.output, `${marker} `, label, format, suffix);
  const opt = (
    option: Option<Value> | undefined,
    state: 'inactive' | 'active' | 'selected' | 'cancelled' | 'disabled',
  ) => {
    if (!option) {
      return '';
    }
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
        return withMarker(color.dim(S_POINTER_INACTIVE), label, color.dim);
    }
  };

  return new SelectPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    output: opts.output,
    initialValue: opts.initialValue,
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);

      switch (this.state) {
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedLines = wrapTextWithPrefix(
            opts.output,
            opt(this.options[this.cursor], 'selected'),
            submitPrefix,
          );
          return `${title}${wrappedLines}\n${promptMilestone('select', opts.testId, 'submit')}`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedLines = wrapTextWithPrefix(
            opts.output,
            opt(this.options[this.cursor], 'cancelled'),
            cancelPrefix,
          );
          return `${title}${wrappedLines}${hasGuide ? `\n${color.gray(S_BAR)}` : ''}\n${promptMilestone('select', opts.testId, 'cancel')}`;
        }
        default: {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          const footer =
            opts.showInstructions === false
              ? []
              : formatInstructionFooter(SELECT_INSTRUCTIONS, hasGuide, opts.output);
          if (hasGuide) {
            footer.push(color.blue(S_BAR_END));
          }
          const prefixEnd = footer.join('\n');
          // Calculate rowPadding: title lines + footer lines (S_BAR_END + trailing newline)
          const titleLineCount = title ? title.split('\n').length : 0;
          const footerLineCount = footer.length + 1; // S_BAR_END + trailing newline (or just trailing newline)
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            cursor: this.cursor,
            options: this.options,
            maxItems: opts.maxItems,
            columnPadding: 2,
            rowPadding: titleLineCount + footerLineCount,
            style: (item, active) =>
              opt(item, item.disabled ? 'disabled' : active ? 'active' : 'inactive'),
          }).join(
            `\n${prefix}`,
          )}\n${prefixEnd}\n${promptMilestone('select', opts.testId, String(this.cursor))}`;
        }
      }
    },
  }).prompt() as Promise<Value | typeof CANCEL_SYMBOL>;
};
