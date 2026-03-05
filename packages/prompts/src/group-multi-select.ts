import { GroupMultiSelectPrompt } from '@clack/core';
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
} from './common.js';
import type { Option } from './select.js';

export interface GroupMultiSelectOptions<Value> extends CommonOptions {
  message: string;
  options: Record<string, Option<Value>[]>;
  initialValues?: Value[];
  required?: boolean;
  cursorAt?: Value;
  selectableGroups?: boolean;
  groupSpacing?: number;
}
export const groupMultiselect = <Value>(opts: GroupMultiSelectOptions<Value>) => {
  const { selectableGroups = true, groupSpacing = 0 } = opts;
  const hasGuide = opts.withGuide ?? false;
  const nestedPrefix = '  ';
  const withMarkerAndPrefix = (
    marker: string,
    prefix: string,
    prefixWidth: number,
    label: string,
    format: (text: string) => string,
    firstLineSuffix = '',
    spacingPrefix = '',
  ) => {
    const lines = label.split('\n');
    const continuationPrefix = `${S_POINTER_INACTIVE} ${' '.repeat(prefixWidth)}`;
    if (lines.length === 1) {
      return `${spacingPrefix}${marker} ${prefix}${format(lines[0])}${firstLineSuffix}`;
    }
    const [firstLine, ...rest] = lines;
    return [
      `${spacingPrefix}${marker} ${prefix}${format(firstLine)}${firstLineSuffix}`,
      ...rest.map((line) => `${continuationPrefix}${format(line)}`),
    ].join('\n');
  };

  const opt = (
    option: Option<Value> & { group: string | boolean },
    state:
      | 'inactive'
      | 'active'
      | 'selected'
      | 'active-selected'
      | 'group-active'
      | 'group-active-selected'
      | 'submitted'
      | 'cancelled',
    options: (Option<Value> & { group: string | boolean })[] = [],
  ) => {
    const label = option.label ?? String(option.value);
    const hint = option.hint ? ` ${color.gray(`(${option.hint})`)}` : '';
    const isItem = typeof option.group === 'string';
    const next = isItem && (options[options.indexOf(option) + 1] ?? { group: true });
    const isLast = isItem && next && next.group === true;
    const branchPrefixRaw = isItem
      ? selectableGroups
        ? `${isLast ? S_BAR_END : S_BAR} `
        : '  '
      : '';
    let spacingPrefix = '';
    if (groupSpacing > 0 && !isItem) {
      const spacingPrefixText = hasGuide ? `\n${color.blue(S_BAR)}` : '\n';
      const spacingSuffix = hasGuide ? ' ' : '';
      spacingPrefix = `${spacingPrefixText.repeat(groupSpacing - 1)}${spacingPrefixText}${spacingSuffix}`;
    }

    if (state === 'cancelled') {
      return color.strikethrough(color.dim(label));
    }
    if (state === 'submitted') {
      return color.dim(label);
    }

    const marker =
      state === 'active' || state === 'active-selected'
        ? color.blue(S_POINTER_ACTIVE)
        : color.dim(S_POINTER_INACTIVE);
    const branchPrefix = color.dim(branchPrefixRaw);
    const hasCheckbox = isItem || selectableGroups;
    const checkboxRaw = hasCheckbox
      ? state === 'active' || state === 'group-active'
        ? S_CHECKBOX_ACTIVE
        : state === 'selected' || state === 'active-selected' || state === 'group-active-selected'
          ? S_CHECKBOX_SELECTED
          : S_CHECKBOX_INACTIVE
      : '';
    const checkbox = hasCheckbox
      ? checkboxRaw === S_CHECKBOX_SELECTED
        ? color.blue(checkboxRaw)
        : checkboxRaw === S_CHECKBOX_ACTIVE
          ? color.blue(checkboxRaw)
          : color.dim(checkboxRaw)
      : '';
    const format =
      state === 'active' || state === 'active-selected'
        ? (text: string) => color.blue(color.bold(text))
        : color.dim;
    const styledPrefix = `${branchPrefix}${hasCheckbox ? `${checkbox} ` : ''}`;
    const prefixWidth = branchPrefixRaw.length + (hasCheckbox ? checkboxRaw.length + 1 : 0);

    return withMarkerAndPrefix(
      marker,
      styledPrefix,
      prefixWidth,
      label,
      format,
      hint,
      spacingPrefix,
    );
  };
  const required = opts.required ?? true;

  return new GroupMultiSelectPrompt({
    options: opts.options,
    signal: opts.signal,
    input: opts.input,
    output: opts.output,
    initialValues: opts.initialValues,
    required,
    cursorAt: opts.cursorAt,
    selectableGroups,
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
      const title = `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${symbol(this.state)} ${opts.message}\n`;
      const value = this.value ?? [];

      switch (this.state) {
        case 'submit': {
          const selectedOptions = this.options
            .filter(({ value: optionValue }) => value.includes(optionValue))
            .map((option) => opt(option, 'submitted'));
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const optionsText =
            selectedOptions.length === 0 ? '' : selectedOptions.join(color.dim(', '));
          return `${title}${submitPrefix}${optionsText}\n\n`;
        }
        case 'cancel': {
          const label = this.options
            .filter(({ value: optionValue }) => value.includes(optionValue))
            .map((option) => opt(option, 'cancelled'))
            .join(color.dim(', '));
          if (!label.trim()) {
            return hasGuide ? `${title}${color.gray(S_BAR)}\n\n` : `${title.trimEnd()}\n\n`;
          }
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return hasGuide
            ? `${title}${cancelPrefix}${label}\n${color.gray(S_BAR)}\n\n`
            : `${title}${cancelPrefix}${label}\n\n`;
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
          return `${title}${prefix}${this.options
            .map((option, i, options) => {
              const selected =
                value.includes(option.value) ||
                (option.group === true && this.isGroupSelected(String(option.value)));
              const active = i === this.cursor;
              const groupActive =
                !active &&
                typeof option.group === 'string' &&
                this.options[this.cursor].value === option.group;
              if (groupActive) {
                return opt(option, selected ? 'group-active-selected' : 'group-active', options);
              }
              if (active && selected) {
                return opt(option, 'active-selected', options);
              }
              if (selected) {
                return opt(option, 'selected', options);
              }
              return opt(option, active ? 'active' : 'inactive', options);
            })
            .join(`\n${prefix}`)}\n${footer}\n`;
        }
        default: {
          const optionsText = this.options
            .map((option, i, options) => {
              const selected =
                value.includes(option.value) ||
                (option.group === true && this.isGroupSelected(String(option.value)));
              const active = i === this.cursor;
              const groupActive =
                !active &&
                typeof option.group === 'string' &&
                this.options[this.cursor].value === option.group;
              let optionText = '';
              if (groupActive) {
                optionText = opt(
                  option,
                  selected ? 'group-active-selected' : 'group-active',
                  options,
                );
              } else if (active && selected) {
                optionText = opt(option, 'active-selected', options);
              } else if (selected) {
                optionText = opt(option, 'selected', options);
              } else {
                optionText = opt(option, active ? 'active' : 'inactive', options);
              }
              const prefix = i !== 0 && !optionText.startsWith('\n') ? '  ' : '';
              return `${prefix}${optionText}`;
            })
            .join(hasGuide ? `\n${color.blue(S_BAR)}` : '\n');
          const optionsPrefix = optionsText.startsWith('\n') ? '' : nestedPrefix;
          const defaultPrefix = hasGuide ? color.blue(S_BAR) : '';
          const defaultSuffix = hasGuide ? color.blue(S_BAR_END) : '';
          return `${title}${defaultPrefix}${optionsPrefix}${optionsText}\n${defaultSuffix}\n`;
        }
      }
    },
  }).prompt() as Promise<Value[] | symbol>;
};
