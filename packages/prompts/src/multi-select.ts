import { MultiSelectPrompt, wrapTextWithPrefix } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  S_BAR,
  S_BAR_END,
  S_CHECKBOX_ACTIVE,
  S_CHECKBOX_INACTIVE,
  S_CHECKBOX_SELECTED,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
  symbol,
  symbolBar,
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
}
const computeLabel = (label: string, format: (text: string) => string) => {
  return label
    .split('\n')
    .map((line) => format(line))
    .join('\n');
};

const withMarkerAndCheckbox = (
  marker: string,
  checkbox: string,
  checkboxWidth: number,
  label: string,
  format: (text: string) => string,
  firstLineSuffix = '',
) => {
  const lines = label.split('\n');
  const continuationPrefix = `${S_POINTER_INACTIVE} ${' '.repeat(checkboxWidth)} `;
  if (lines.length === 1) {
    return `${marker} ${checkbox} ${format(lines[0])}${firstLineSuffix}`;
  }
  const [firstLine, ...rest] = lines;
  return [
    `${marker} ${checkbox} ${format(firstLine)}${firstLineSuffix}`,
    ...rest.map((line) => `${continuationPrefix}${format(line)}`),
  ].join('\n');
};

export const multiselect = <Value>(opts: MultiSelectOptions<Value>) => {
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
        hint,
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
    output: opts.output,
    initialValues: opts.initialValues,
    required,
    cursorAt: opts.cursorAt,
    validate(selected: Value[] | undefined) {
      if (required && (selected === undefined || selected.length === 0)) {
        return `Please select at least one option.\n${color.reset(
          color.dim(
            `Press ${color.gray(color.bgWhite(color.inverse(' space ')))} to select, ${color.gray(
              color.bgWhite(color.inverse(' enter ')),
            )} to submit`,
          ),
        )}`;
      }
    },
    render() {
      const hasGuide = opts.withGuide ?? false;
      const nestedPrefix = '  ';
      const formatMessageLines = (message: string) => {
        const lines = message.split('\n');
        return lines
          .map((line, index) => `${index === 0 ? `${symbol(this.state)} ` : nestedPrefix}${line}`)
          .join('\n');
      };
      const wrappedMessage = hasGuide
        ? wrapTextWithPrefix(
            opts.output,
            opts.message,
            `${symbolBar(this.state)} `,
            `${symbol(this.state)} `,
          )
        : formatMessageLines(opts.message);
      const title = `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${wrappedMessage}\n`;
      const value = this.value ?? [];

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
          const footer = hasGuide
            ? this.error
                .split('\n')
                .map((ln, i) =>
                  i === 0 ? `${color.yellow(S_BAR_END)} ${color.yellow(ln)}` : `  ${ln}`,
                )
                .join('\n')
            : `${nestedPrefix}${color.yellow(this.error)}`;
          // Calculate rowPadding: title lines + footer lines (error message + trailing newline)
          const titleLineCount = title.split('\n').length;
          const footerLineCount = footer.split('\n').length + 1; // footer + trailing newline
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            options: this.options,
            cursor: this.cursor,
            maxItems: opts.maxItems,
            columnPadding: prefix.length,
            rowPadding: titleLineCount + footerLineCount,
            style: styleOption,
          }).join(`\n${prefix}`)}\n${footer}\n`;
        }
        default: {
          const prefix = hasGuide ? `${color.blue(S_BAR)} ` : nestedPrefix;
          // Calculate rowPadding: title lines + footer lines (S_BAR_END + trailing newline)
          const titleLineCount = title.split('\n').length;
          const footerLineCount = hasGuide ? 2 : 1; // S_BAR_END + trailing newline
          return `${title}${prefix}${limitOptions({
            output: opts.output,
            options: this.options,
            cursor: this.cursor,
            maxItems: opts.maxItems,
            columnPadding: prefix.length,
            rowPadding: titleLineCount + footerLineCount,
            style: styleOption,
          }).join(`\n${prefix}`)}\n${hasGuide ? color.blue(S_BAR_END) : ''}\n`;
        }
      }
    },
  }).prompt() as Promise<Value[] | symbol>;
};
