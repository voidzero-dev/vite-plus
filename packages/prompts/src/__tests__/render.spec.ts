import { beforeEach, describe, expect, it, vi } from 'vitest';

type PromptConfig = {
  render: (this: Record<string, unknown>) => string;
};

const captured: {
  select?: PromptConfig;
  multiSelect?: PromptConfig;
  groupMultiSelect?: PromptConfig;
  autocomplete?: PromptConfig;
  confirm?: PromptConfig;
  selectKey?: PromptConfig;
  text?: PromptConfig;
  password?: PromptConfig;
} = {};

class SelectPrompt<_Value> {
  constructor(config: PromptConfig) {
    captured.select = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class MultiSelectPrompt<_Value> {
  constructor(config: PromptConfig) {
    captured.multiSelect = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class GroupMultiSelectPrompt<_Value> {
  constructor(config: PromptConfig) {
    captured.groupMultiSelect = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class AutocompletePrompt<_Value> {
  constructor(config: PromptConfig) {
    captured.autocomplete = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class ConfirmPrompt {
  constructor(config: PromptConfig) {
    captured.confirm = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class SelectKeyPrompt<_Value> {
  constructor(config: PromptConfig) {
    captured.selectKey = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class TextPrompt {
  constructor(config: PromptConfig) {
    captured.text = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

class PasswordPrompt {
  constructor(config: PromptConfig) {
    captured.password = config;
  }

  prompt() {
    return Promise.resolve(Symbol('cancel'));
  }
}

vi.mock('@clack/core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@clack/core')>();
  return {
    ...actual,
    getColumns: (output?: { columns?: number }) => output?.columns ?? 80,
    getRows: (output?: { rows?: number }) => output?.rows ?? 24,
    SelectPrompt,
    MultiSelectPrompt,
    GroupMultiSelectPrompt,
    AutocompletePrompt,
    ConfirmPrompt,
    SelectKeyPrompt,
    TextPrompt,
    PasswordPrompt,
  };
});

// oxlint-disable-next-line no-control-regex
const stripAnsi = (value: string): string => value.replaceAll(/\x1b\[[0-9;]*m/gu, '');
const normalize = (value: string): string => stripAnsi(value).replaceAll('\r\n', '\n');

const renderWith = (config: PromptConfig | undefined, ctx: Record<string, unknown>): string => {
  if (config === undefined) {
    throw new Error('Prompt config was not captured');
  }
  return normalize(config.render.call(ctx));
};

beforeEach(() => {
  captured.select = undefined;
  captured.multiSelect = undefined;
  captured.groupMultiSelect = undefined;
  captured.autocomplete = undefined;
  captured.confirm = undefined;
  captured.selectKey = undefined;
  captured.text = undefined;
  captured.password = undefined;
});

describe('prompt renderers', () => {
  it('renders select with pointer markers and aligned multiline labels', async () => {
    const { select } = await import('../select.js');
    const options = [
      { value: 'alpha', label: 'Alpha\nSecond line', hint: 'recommended' },
      { value: 'beta', label: 'Beta' },
    ];
    void select({
      message: 'Choose an option',
      options,
    });
    expect(
      renderWith(captured.select, {
        state: 'active',
        options,
        cursor: 0,
      }),
    ).toMatchSnapshot();
  });

  it('renders multiselect with cursor marker plus checkbox state', async () => {
    const { multiselect } = await import('../multi-select.js');
    const options = [
      { value: 'alpha', label: 'Alpha\nSecond line', hint: 'recommended' },
      { value: 'beta', label: 'Beta' },
      { value: 'gamma', label: 'Gamma' },
    ];
    void multiselect({
      message: 'Choose multiple',
      options,
    });
    expect(
      renderWith(captured.multiSelect, {
        state: 'active',
        options,
        cursor: 1,
        value: ['beta'],
      }),
    ).toMatchSnapshot();
  });

  it('renders grouped multiselect with marker, tree branch, and checkbox alignment', async () => {
    const { groupMultiselect } = await import('../group-multi-select.js');
    void groupMultiselect({
      message: 'Grouped choices',
      options: {
        Fruits: [
          { value: 'apple', label: 'Apple\nGreen', hint: 'fresh' },
          { value: 'banana', label: 'Banana' },
        ],
        Tools: [{ value: 'hammer', label: 'Hammer' }],
      },
      selectableGroups: true,
      groupSpacing: 1,
    });
    const renderedOptions = [
      { value: 'Fruits', label: 'Fruits', group: true },
      { value: 'apple', label: 'Apple\nGreen', hint: 'fresh', group: 'Fruits' },
      { value: 'banana', label: 'Banana', group: 'Fruits' },
      { value: 'Tools', label: 'Tools', group: true },
      { value: 'hammer', label: 'Hammer', group: 'Tools' },
    ];
    expect(
      renderWith(captured.groupMultiSelect, {
        state: 'active',
        options: renderedOptions,
        cursor: 2,
        value: ['banana'],
        isGroupSelected: () => false,
      }),
    ).toMatchSnapshot();
  });

  it('renders autocomplete with pointer markers and focused hint', async () => {
    const { autocomplete } = await import('../autocomplete.js');
    const options = [
      { value: 'alpha', label: 'Alpha\nSecond line', hint: 'recommended' },
      { value: 'beta', label: 'Beta' },
    ];
    void autocomplete({
      message: 'Search options',
      options,
      placeholder: 'type to filter',
    });
    expect(
      renderWith(captured.autocomplete, {
        state: 'active',
        options,
        filteredOptions: options,
        selectedValues: [],
        focusedValue: 'alpha',
        userInput: '',
        userInputWithCursor: '|',
        cursor: 0,
        isNavigating: true,
      }),
    ).toMatchSnapshot();
  });

  it('renders autocomplete multiselect with pointer markers plus checkboxes', async () => {
    const { autocompleteMultiselect } = await import('../autocomplete.js');
    const options = [
      { value: 'alpha', label: 'Alpha\nSecond line', hint: 'recommended' },
      { value: 'beta', label: 'Beta' },
    ];
    void autocompleteMultiselect({
      message: 'Search and select',
      options,
      placeholder: 'type to filter',
    });
    expect(
      renderWith(captured.autocomplete, {
        state: 'active',
        options,
        filteredOptions: options,
        selectedValues: ['beta'],
        focusedValue: 'beta',
        userInput: '',
        userInputWithCursor: '|',
        cursor: 1,
        isNavigating: true,
      }),
    ).toMatchSnapshot();
  });

  it('renders confirm and select-key in pointer style', async () => {
    const [{ confirm }, { selectKey }] = await Promise.all([
      import('../confirm.js'),
      import('../select-key.js'),
    ]);
    void confirm({ message: 'Proceed?' });
    const confirmOutput = renderWith(captured.confirm, {
      state: 'active',
      value: true,
    });

    const keyOptions = [
      { value: 'a', label: 'Add item' },
      { value: 'r', label: 'Remove item' },
    ];
    void selectKey({
      message: 'Pick shortcut',
      options: keyOptions,
    });
    const selectKeyOutput = renderWith(captured.selectKey, {
      state: 'active',
      options: keyOptions,
      cursor: 0,
    });

    expect(`${confirmOutput}\n---\n${selectKeyOutput}`).toMatchSnapshot();
  });

  it('renders submitted prompts without extra blank lines', async () => {
    const [{ multiselect }, { confirm }, { text }] = await Promise.all([
      import('../multi-select.js'),
      import('../confirm.js'),
      import('../text.js'),
    ]);

    const multiOptions = [
      { value: 'alpha', label: 'Alpha' },
      { value: 'beta', label: 'Beta' },
    ];
    void multiselect({
      message: 'Choose multiple',
      options: multiOptions,
    });
    const multiselectOutput = renderWith(captured.multiSelect, {
      state: 'submit',
      options: multiOptions,
      value: ['beta'],
    });

    void confirm({ message: 'Proceed?' });
    const confirmOutput = renderWith(captured.confirm, {
      state: 'submit',
      value: true,
    });

    void text({ message: 'Project name' });
    const textOutput = renderWith(captured.text, {
      state: 'submit',
      value: 'acme-web',
    });

    expect(`${multiselectOutput}\n---\n${confirmOutput}\n---\n${textOutput}`).toMatchSnapshot();
  });
});

describe('renderer edge cases', () => {
  it('hides inactive hints and supports hiding keyboard instructions', async () => {
    const { select } = await import('../select.js');
    const options = [
      { value: 'a', label: 'Alpha', hint: 'Focused hint' },
      { value: 'b', label: 'Beta', hint: 'Hidden hint' },
    ];
    void select({ message: 'Choose', options, showInstructions: false });
    const result = renderWith(captured.select, { state: 'active', options, cursor: 0 });
    expect(result).toContain('Focused hint');
    expect(result).not.toContain('Hidden hint');
    expect(result).not.toContain('Enter');
  });

  it('wraps headings, labels, and instructions within visible columns with guides', async () => {
    const { select } = await import('../select.js');
    const { Writable } = await import('node:stream');
    const output = Object.assign(
      new Writable({
        write(_chunk, _encoding, done) {
          done();
        },
      }),
      { columns: 24 },
    );
    const options = [{ value: 'a', label: '日本語 long option label', hint: 'Long focused hint' }];
    void select({
      message: 'A very long question with 日本語 text?',
      options,
      output,
      withGuide: true,
    });
    const result = renderWith(captured.select, { state: 'active', options, cursor: 0 });
    const { default: width } = await import('fast-string-width');
    expect(result.split('\n').every((line) => width(line) <= 24)).toBe(true);
    expect(result).toContain('│');
    expect(result).toContain('›');
  });

  it('renders empty lists on submit and cancellation without dereferencing a missing option', async () => {
    const [{ select }, { selectKey }, { groupMultiselect }] = await Promise.all([
      import('../select.js'),
      import('../select-key.js'),
      import('../group-multi-select.js'),
    ]);
    void select({ message: 'Choose', options: [] });
    void selectKey({ message: 'Choose', options: [] });
    void groupMultiselect({ message: 'Choose', options: {}, required: false });
    for (const state of ['submit', 'cancel', 'active']) {
      const ctx = { state, options: [], cursor: 0, value: [], isGroupSelected: () => false };
      expect(() => renderWith(captured.select, ctx)).not.toThrow();
      expect(() => renderWith(captured.selectKey, ctx)).not.toThrow();
      expect(() => renderWith(captured.groupMultiSelect, ctx)).not.toThrow();
    }
  });
});
