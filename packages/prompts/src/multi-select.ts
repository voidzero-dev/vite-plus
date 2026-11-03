import type { CANCEL_SYMBOL } from '@clack/core';
import { MultiSelectPrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  formatInstructionFooter,
  MULTISELECT_INSTRUCTIONS,
  promptTitle,
  optionText,
  wrapTextWithPrefix,
  S_BAR,
  S_BAR_END,
  S_CHECKBOX_ACTIVE,
  S_CHECKBOX_INACTIVE,
  S_CHECKBOX_SELECTED,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
} from './common.js';
import { limitOptions } from './limit-options.js';
import type { Option } from './select.js';

export interface MultiSelectOptions<Value> extends CommonOptions {
  message: string;
  options: Option<Value>[];
  initialValues?: Value[];
  maxItems?: number;
  required?: boolean;
  cursorAt?: Value;
  showInstructions?: boolean;
}
const computeLabel = (label: string, format: (text: string) => string) => {
  return label
    .split('\n')
    .map((line) => format(line))
    .join('\n');
};

export const multiselect = <Value>(opts: MultiSelectOptions<Value>) => {
  const withMarkerAndCheckbox = (
    marker: string,
    checkbox: string,
    _width: number,
    label: string,
    format: (text: string) => string,
    suffix = '',
  ) => optionText(opts.output, `${marker} ${checkbox} `, label, format, suffix);
  const opt = (
    option: Option<Value>,
    state:
      | 'inactive'
      | 'active'
      | 'selected'
      | 'active-selected'
      | 'submitted'
      | 'cancelled'
      | 'disabled',
  ) => {
    const label = option.label ?? String(option.value);
    const hint = option.hint ? ` ${color.gray(`(${option.hint})`)}` : '';
    if (state === 'disabled') {
      return withMarkerAndCheckbox(
        color.gray(S_POINTER_INACTIVE),
        color.gray(S_CHECKBOX_INACTIVE),
        S_CHECKBOX_INACTIVE.length,
        label,
        (str) => color.strikethrough(color.gray(str)),
        option.hint ? ` ${color.dim(`(${option.hint ?? 'disabled'})`)}` : '',
      );
    }
    if (state === 'active') {
      return withMarkerAndCheckbox(
        color.blue(S_POINTER_ACTIVE),
        color.blue(S_CHECKBOX_ACTIVE),
        S_CHECKBOX_ACTIVE.length,
        label,
        (text) => color.blue(color.bold(text)),
        hint,
      );
    }
    if (state === 'selected') {
      return withMarkerAndCheckbox(
        color.dim(S_POINTER_INACTIVE),
        color.blue(S_CHECKBOX_SELECTED),
        S_CHECKBOX_SELECTED.length,
        label,
        color.dim,
      );
    }
    if (state === 'cancelled') {
      return computeLabel(label, (text) => color.strikethrough(color.dim(text)));
    }
    if (state === 'active-selected') {
      return withMarkerAndCheckbox(
        color.blue(S_POINTER_ACTIVE),
        color.blue(S_CHECKBOX_SELECTED),
        S_CHECKBOX_SELECTED.length,
        label,
        (text) => color.blue(color.bold(text)),
        hint,
      );
    }
    if (state === 'submitted') {
      return computeLabel(label, color.dim);
    }
    return withMarkerAndCheckbox(
      color.dim(S_POINTER_INACTIVE),
      color.dim(S_CHECKBOX_INACTIVE),
      S_CHECKBOX_INACTIVE.length,
      label,
      color.dim,
    );
  };
  const required = opts.required ?? true;

  return new MultiSelectPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    accessible: opts.accessible,
    output: opts.output,
    initialValues: opts.initialValues,
    required,
    cursorAt: opts.cursorAt,
    validate(selected: Value[] | undefined) {
      if (required && (selected === undefined || selected.length === 0)) {
        return 'Please select at least one option.';
      }
      return undefined;
    },
    render() {
      const hasGuide = getGuide(opts);
      const nestedPrefix = '  ';
      const title = promptTitle(opts.message, this.state, opts);
      const value = this.value ?? [];
      const instructions =
        opts.showInstructions === false
          ? []
          : formatInstructionFooter(MULTISELECT_INSTRUCTIONS, hasGuide, opts.output);
      const hint = instructions.join('\n');

      const styleOption = (option: Option<Value>, active: boolean) => {
        if (option.disabled) {
          return opt(option, 'disabled');
        }
        const selected = value.includes(option.value);
        if (active && selected) {
          return opt(option, 'active-selected');
        }
        if (selected) {
          return opt(option, 'selected');
        }
        return opt(option, active ? 'active' : 'inactive');
      };

      switch (this.state) {
        case 'submit': {
          const submitText =
            this.options
              .filter(({ value: optionValue }) => value.includes(optionValue))
              .map((option) => opt(option, 'submitted'))
              .join(color.dim(', ')) || color.dim('none');
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedSubmitText = wrapTextWithPrefix(opts.output, submitText, submitPrefix);
          return `${title}${wrappedSubmitText}\n`;
        }
        case 'cancel': {
          const label = this.options
            .filter(({ value: optionValue }) => value.includes(optionValue))
            .map((option) => opt(option, 'cancelled'))
            .join(color.dim(', '));
          if (label.trim() === '') {
            return hasGuide ? `${title}${color.gray(S_BAR)}\n` : `${title.trimEnd()}\n`;
          }
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const wrappedLabel = wrapTextWithPrefix(opts.output, label, cancelPrefix);
          return hasGuide
            ? `${title}${wrappedLabel}\n${color.gray(S_BAR)}\n`
            : `${title}${wrappedLabel}\n`;
        }
        case 'error': {
          const prefix = hasGuide ? `${color.yellow(S_BAR)} ` : nestedPrefix;
          const footer = wrapTextWithPrefix(
            opts.output,
            color.yellow(this.error),
            hasGuide ? `${color.yellow(S_BAR_END)} ` : nestedPrefix,
          );
          // Calculate rowPadding: title lines + footer lines (error message + trailing newline)
          const titleLineCount = title.split('\n').length;
          const footerLineCount = footer.split('\n').length + instructions.length + 1; // footer + trailing newline
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            options: this.options,
            cursor: this.cursor,
            maxItems: opts.maxItems,
            columnPadding: 2,
            rowPadding: titleLineCount + footerLineCount,
            style: styleOption,
          }).join(`\n${prefix}`)}\n${hint}\n${footer}\n`;
        }
        default: {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          // Calculate rowPadding: title lines + footer lines (S_BAR_END + trailing newline)
          const titleLineCount = title.split('\n').length;
          const footerLineCount = instructions.length + (hasGuide ? 2 : 1); // S_BAR_END + trailing newline
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            options: this.options,
            cursor: this.cursor,
            maxItems: opts.maxItems,
            columnPadding: 2,
            rowPadding: titleLineCount + footerLineCount,
            style: styleOption,
          }).join(`\n${prefix}`)}\n${hint}${hasGuide ? `\n${color.blue(S_BAR_END)}` : ''}\n`;
        }
      }
    },
  }).prompt() as Promise<Value[] | typeof CANCEL_SYMBOL>;
};
