import type { CANCEL_SYMBOL } from '@clack/core';
import { GroupMultiSelectPrompt } from '@clack/core';
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

export interface GroupMultiSelectOptions<Value> extends CommonOptions {
  message: string;
  options: Record<string, Option<Value>[]>;
  initialValues?: Value[];
  required?: boolean;
  cursorAt?: Value;
  selectableGroups?: boolean;
  groupSpacing?: number;
  maxItems?: number;
  showInstructions?: boolean;
}
export const groupMultiselect = <Value>(opts: GroupMultiSelectOptions<Value>) => {
  const { selectableGroups = true, groupSpacing = 0 } = opts;
  const hasGuide = getGuide(opts);
  const nestedPrefix = '  ';
  // eslint-disable-next-line unicorn/consistent-function-scoping -- kept inline for readability
  const withMarkerAndPrefix = (
    marker: string,
    prefix: string,
    _prefixWidth: number,
    label: string,
    format: (text: string) => string,
    firstLineSuffix = '',
    spacingPrefix = '',
  ) => {
    return (
      spacingPrefix + optionText(opts.output, `${marker} ${prefix}`, label, format, firstLineSuffix)
    );
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
    const hint =
      option.hint && (state === 'active' || state === 'active-selected')
        ? ` ${color.gray(`(${option.hint})`)}`
        : '';
    const isItem = typeof option.group === 'string';
    const next = isItem && (options[options.indexOf(option) + 1] ?? { group: true });
    const isLast = isItem && next && next.group === true;
    const branchPrefixRaw = isItem
      ? selectableGroups
        ? `${isLast ? S_BAR_END : S_BAR} `
        : '  '
      : '';
    let spacingPrefix = '';
    if (groupSpacing > 0 && !isItem && options.indexOf(option) > 0) {
      spacingPrefix = '\n'.repeat(groupSpacing);
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
    accessible: opts.accessible,
    output: opts.output,
    initialValues: opts.initialValues,
    required,
    cursorAt: opts.cursorAt,
    selectableGroups,
    validate(selected: Value[] | undefined) {
      if (required && (selected === undefined || selected.length === 0)) {
        return 'Please select at least one option.';
      }
      return undefined;
    },
    render() {
      const title = promptTitle(opts.message, this.state, opts);
      const value = this.value ?? [];

      switch (this.state) {
        case 'submit': {
          const selectedOptions = this.options
            .filter(({ value: optionValue }) => value.includes(optionValue))
            .map((option) => opt(option, 'submitted'));
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          const optionsText =
            selectedOptions.length === 0 ? '' : selectedOptions.join(color.dim(', '));
          return `${title}${submitPrefix}${optionsText}\n`;
        }
        case 'cancel': {
          const label = this.options
            .filter(({ value: optionValue }) => value.includes(optionValue))
            .map((option) => opt(option, 'cancelled'))
            .join(color.dim(', '));
          if (!label.trim()) {
            return hasGuide ? `${title}${color.gray(S_BAR)}\n` : `${title.trimEnd()}\n`;
          }
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return hasGuide
            ? `${title}${cancelPrefix}${label}\n${color.gray(S_BAR)}\n`
            : `${title}${cancelPrefix}${label}\n`;
        }
        default: {
          const prefix = hasGuide
            ? `${this.state === 'error' ? color.yellow(S_BAR) : color.blue(S_BAR)} `
            : nestedPrefix;
          const footer =
            opts.showInstructions === false
              ? []
              : formatInstructionFooter(MULTISELECT_INSTRUCTIONS, hasGuide, opts.output);
          if (this.state === 'error') {
            footer.push(
              wrapTextWithPrefix(
                opts.output,
                color.yellow(this.error),
                hasGuide ? `${color.yellow(S_BAR_END)} ` : nestedPrefix,
              ),
            );
          } else if (hasGuide) {
            footer.push(color.blue(S_BAR_END));
          }
          const optionsText = limitOptions({
            output: opts.output,
            options: this.options,
            cursor: this.cursor,
            maxItems: opts.maxItems,
            columnPadding: 2,
            rowPadding: title.split('\n').length + footer.length + 1,
            style: (option, active) => {
              const selected =
                value.includes(option.value) ||
                (option.group === true && this.isGroupSelected(String(option.value)));
              const groupActive =
                !active &&
                typeof option.group === 'string' &&
                this.options[this.cursor]?.value === option.group;
              return opt(
                option,
                groupActive
                  ? selected
                    ? 'group-active-selected'
                    : 'group-active'
                  : active
                    ? selected
                      ? 'active-selected'
                      : 'active'
                    : selected
                      ? 'selected'
                      : 'inactive',
                this.options,
              );
            },
          })
            .map((line) => `${line ? prefix : hasGuide ? prefix.trimEnd() : ''}${line}`)
            .join('\n');
          return `${title}${optionsText}\n${footer.join('\n')}\n`;
        }
      }
    },
  }).prompt() as Promise<Value[] | typeof CANCEL_SYMBOL>;
};
