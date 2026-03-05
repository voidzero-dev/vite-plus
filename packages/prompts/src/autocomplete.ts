import { AutocompletePrompt } from '@clack/core';
import color from 'picocolors';

import {
  type CommonOptions,
  S_BAR,
  S_BAR_END,
  S_CHECKBOX_INACTIVE,
  S_CHECKBOX_SELECTED,
  S_POINTER_ACTIVE,
  S_POINTER_INACTIVE,
  symbol,
} from './common.js';
import { limitOptions } from './limit-options.js';
import type { Option } from './select.js';

function getLabel<T>(option: Option<T>) {
  return option.label ?? String(option.value ?? '');
}

function getFilteredOption<T>(searchText: string, option: Option<T>): boolean {
  if (!searchText) {
    return true;
  }
  const label = (option.label ?? String(option.value ?? '')).toLowerCase();
  const hint = (option.hint ?? '').toLowerCase();
  const value = String(option.value).toLowerCase();
  const term = searchText.toLowerCase();

  return label.includes(term) || hint.includes(term) || value.includes(term);
}

function getSelectedOptions<T>(values: T[], options: Option<T>[]): Option<T>[] {
  const results: Option<T>[] = [];

  for (const option of options) {
    if (values.includes(option.value)) {
      results.push(option);
    }
  }

  return results;
}

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

const withMarkerAndIndicator = (
  marker: string,
  indicator: string,
  indicatorWidth: number,
  label: string,
  format: (text: string) => string,
  firstLineSuffix = '',
) => {
  const lines = label.split('\n');
  const continuationPrefix = `${S_POINTER_INACTIVE} ${' '.repeat(indicatorWidth)} `;
  if (lines.length === 1) {
    return `${marker} ${indicator} ${format(lines[0])}${firstLineSuffix}`;
  }
  const [firstLine, ...rest] = lines;
  return [
    `${marker} ${indicator} ${format(firstLine)}${firstLineSuffix}`,
    ...rest.map((line) => `${continuationPrefix}${format(line)}`),
  ].join('\n');
};

interface AutocompleteSharedOptions<Value> extends CommonOptions {
  /**
   * The message to display to the user.
   */
  message: string;
  /**
   * Available options for the autocomplete prompt.
   */
  options: Option<Value>[] | ((this: AutocompletePrompt<Option<Value>>) => Option<Value>[]);
  /**
   * Maximum number of items to display at once.
   */
  maxItems?: number;
  /**
   * Placeholder text to display when no input is provided.
   */
  placeholder?: string;
  /**
   * Validates the value
   */
  validate?: (value: Value | Value[] | undefined) => string | Error | undefined;
  /**
   * Custom filter function to match options against search input.
   * If not provided, a default filter that matches label, hint, and value is used.
   */
  filter?: (search: string, option: Option<Value>) => boolean;
}

export interface AutocompleteOptions<Value> extends AutocompleteSharedOptions<Value> {
  /**
   * The initial selected value.
   */
  initialValue?: Value;
  /**
   * The initial user input
   */
  initialUserInput?: string;
}

export const autocomplete = <Value>(opts: AutocompleteOptions<Value>) => {
  const prompt = new AutocompletePrompt({
    options: opts.options,
    initialValue: opts.initialValue ? [opts.initialValue] : undefined,
    initialUserInput: opts.initialUserInput,
    filter:
      opts.filter ??
      ((search: string, opt: Option<Value>) => {
        return getFilteredOption(search, opt);
      }),
    signal: opts.signal,
    input: opts.input,
    output: opts.output,
    validate: opts.validate,
    render() {
      const hasGuide = opts.withGuide ?? false;
      const nestedPrefix = '  ';
      // Title and message display
      const headings = hasGuide
        ? [color.gray(S_BAR), `${symbol(this.state)} ${opts.message}`]
        : [`${symbol(this.state)} ${opts.message}`];
      const userInput = this.userInput;
      const options = this.options;
      const placeholder = opts.placeholder;
      const showPlaceholder = userInput === '' && placeholder !== undefined;

      // Handle different states
      switch (this.state) {
        case 'submit': {
          // Show selected value
          const selected = getSelectedOptions(this.selectedValues, options);
          const label = selected.length > 0 ? color.dim(selected.map(getLabel).join(', ')) : '';
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${headings.join('\n')}\n${submitPrefix}${label}\n\n`;
        }

        case 'cancel': {
          const userInputText = userInput ? color.strikethrough(color.dim(userInput)) : '';
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : nestedPrefix;
          return `${headings.join('\n')}\n${cancelPrefix}${userInputText}\n\n`;
        }

        default: {
          const barColor = this.state === 'error' ? color.yellow : color.blue;
          const guidePrefix = hasGuide ? `${barColor(S_BAR)} ` : nestedPrefix;
          const guidePrefixEnd = hasGuide ? barColor(S_BAR_END) : '';
          // Display cursor position - show plain text in navigation mode
          let searchText = '';
          if (this.isNavigating || showPlaceholder) {
            const searchTextValue = showPlaceholder ? placeholder : userInput;
            searchText = searchTextValue !== '' ? ` ${color.dim(searchTextValue)}` : '';
          } else {
            searchText = ` ${this.userInputWithCursor}`;
          }

          // Show match count if filtered
          const matches =
            this.filteredOptions.length !== options.length
              ? color.dim(
                  ` (${this.filteredOptions.length} match${this.filteredOptions.length === 1 ? '' : 'es'})`,
                )
              : '';

          // No matches message
          const noResults =
            this.filteredOptions.length === 0 && userInput
              ? [`${guidePrefix}${color.yellow('No matches found')}`]
              : [];

          const validationError =
            this.state === 'error' ? [`${guidePrefix}${color.yellow(this.error)}`] : [];

          if (hasGuide) {
            headings.push(guidePrefix.trimEnd());
          }
          headings.push(
            `${guidePrefix}${color.dim('Search:')}${searchText}${matches}`,
            ...noResults,
            ...validationError,
          );

          // Show instructions
          const instructions = [
            `${color.dim('↑/↓')} to select`,
            `${color.dim('Enter:')} confirm`,
            `${color.dim('Type:')} to search`,
          ];

          const footers = [`${guidePrefix}${instructions.join(' • ')}`, guidePrefixEnd];

          // Render options with selection
          const displayOptions =
            this.filteredOptions.length === 0
              ? []
              : limitOptions({
                  cursor: this.cursor,
                  options: this.filteredOptions,
                  columnPadding: hasGuide ? 2 : 2,
                  rowPadding: headings.length + footers.length,
                  style: (option, active) => {
                    const label = getLabel(option);
                    const hint =
                      option.hint && option.value === this.focusedValue
                        ? color.gray(` (${option.hint})`)
                        : '';

                    return active
                      ? withMarker(
                          color.blue(S_POINTER_ACTIVE),
                          label,
                          (text) => color.blue(color.bold(text)),
                          hint,
                        )
                      : withMarker(color.dim(S_POINTER_INACTIVE), label, color.dim, hint);
                  },
                  maxItems: opts.maxItems,
                  output: opts.output,
                });

          // Return the formatted prompt
          return [
            ...headings,
            ...displayOptions.map((option) => `${guidePrefix}${option}`),
            ...footers,
          ].join('\n');
        }
      }
    },
  });

  // Return the result or cancel symbol
  return prompt.prompt() as Promise<Value | symbol>;
};

// Type definition for the autocompleteMultiselect component
export interface AutocompleteMultiSelectOptions<Value> extends AutocompleteSharedOptions<Value> {
  /**
   * The initial selected values
   */
  initialValues?: Value[];
  /**
   * If true, at least one option must be selected
   */
  required?: boolean;
}

/**
 * Integrated autocomplete multiselect - combines type-ahead filtering with multiselect in one UI
 */
export const autocompleteMultiselect = <Value>(opts: AutocompleteMultiSelectOptions<Value>) => {
  const formatOption = (
    option: Option<Value>,
    active: boolean,
    selectedValues: Value[],
    focusedValue: Value | undefined,
  ) => {
    const isSelected = selectedValues.includes(option.value);
    const label = option.label ?? String(option.value ?? '');
    const hint =
      option.hint && focusedValue !== undefined && option.value === focusedValue
        ? color.gray(` (${option.hint})`)
        : '';
    const checkboxRaw = isSelected ? S_CHECKBOX_SELECTED : S_CHECKBOX_INACTIVE;
    const checkbox = isSelected ? color.blue(checkboxRaw) : color.dim(checkboxRaw);
    const marker = active ? color.blue(S_POINTER_ACTIVE) : color.dim(S_POINTER_INACTIVE);
    return withMarkerAndIndicator(
      marker,
      checkbox,
      checkboxRaw.length,
      label,
      active ? (text) => color.blue(color.bold(text)) : color.dim,
      hint,
    );
  };

  // Create text prompt which we'll use as foundation
  const prompt = new AutocompletePrompt<Option<Value>>({
    options: opts.options,
    multiple: true,
    filter:
      opts.filter ??
      ((search, opt) => {
        return getFilteredOption(search, opt);
      }),
    validate: () => {
      if (opts.required && prompt.selectedValues.length === 0) {
        return 'Please select at least one item';
      }
      return undefined;
    },
    initialValue: opts.initialValues,
    signal: opts.signal,
    input: opts.input,
    output: opts.output,
    render() {
      const hasGuide = opts.withGuide ?? false;
      const nestedPrefix = '  ';
      // Title and symbol
      const title = `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${symbol(this.state)} ${opts.message}\n`;

      // Selection counter
      const userInput = this.userInput;
      const placeholder = opts.placeholder;
      const showPlaceholder = userInput === '' && placeholder !== undefined;

      // Search input display
      const searchText =
        this.isNavigating || showPlaceholder
          ? color.dim(showPlaceholder ? placeholder : userInput) // Just show plain text when in navigation mode
          : this.userInputWithCursor;

      const options = this.options;

      const matches =
        this.filteredOptions.length !== options.length
          ? color.dim(
              ` (${this.filteredOptions.length} match${this.filteredOptions.length === 1 ? '' : 'es'})`,
            )
          : '';

      // Render prompt state
      switch (this.state) {
        case 'submit': {
          const submitPrefix = hasGuide ? `${color.gray(S_BAR)} ` : '';
          const finalPrefix = hasGuide ? submitPrefix : nestedPrefix;
          return `${title}${finalPrefix}${color.dim(`${this.selectedValues.length} items selected`)}\n\n`;
        }
        case 'cancel': {
          const cancelPrefix = hasGuide ? `${color.gray(S_BAR)} ` : '';
          const finalPrefix = hasGuide ? cancelPrefix : nestedPrefix;
          return `${title}${finalPrefix}${color.strikethrough(color.dim(userInput))}\n\n`;
        }
        default: {
          const barColor = this.state === 'error' ? color.yellow : color.blue;
          const prefix = hasGuide ? `${barColor(S_BAR)} ` : nestedPrefix;
          const footerEnd = hasGuide ? [barColor(S_BAR_END)] : [];
          // Instructions
          const instructions = [
            `${color.dim('↑/↓')} to navigate`,
            `${color.dim(this.isNavigating ? 'Space/Tab:' : 'Tab:')} select`,
            `${color.dim('Enter:')} confirm`,
            `${color.dim('Type:')} to search`,
          ];

          // No results message
          const noResults =
            this.filteredOptions.length === 0 && userInput
              ? [`${prefix}${color.yellow('No matches found')}`]
              : [];

          const errorMessage =
            this.state === 'error' ? [`${prefix}${color.yellow(this.error)}`] : [];

          // Calculate header and footer line counts for rowPadding
          const headerLines = [
            ...title.trimEnd().split('\n'),
            `${prefix}${color.dim('Search:')} ${searchText}${matches}`,
            ...noResults,
            ...errorMessage,
          ];
          const footerLines = [`${prefix}${instructions.join(' • ')}`, ...footerEnd];

          // Get limited options for display
          const displayOptions = limitOptions({
            cursor: this.cursor,
            options: this.filteredOptions,
            style: (option, active) =>
              formatOption(option, active, this.selectedValues, this.focusedValue),
            maxItems: opts.maxItems,
            output: opts.output,
            rowPadding: headerLines.length + footerLines.length,
          });

          // Build the prompt display
          return [
            ...headerLines,
            ...displayOptions.map((option) => `${prefix}${option}`),
            ...footerLines,
          ].join('\n');
        }
      }
    },
  });

  // Return the result or cancel symbol
  return prompt.prompt() as Promise<Value[] | symbol>;
};
