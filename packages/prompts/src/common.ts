import type { Readable, Writable } from 'node:stream';

import { type State, settings, updateSettings, getColumns } from '@clack/core';
import stringWidth from 'fast-string-width';
import { wrapAnsi } from 'fast-wrap-ansi';
import isUnicodeSupported from 'is-unicode-supported';
import color from 'picocolors';

// Vite+ defaults to frameless prompts. Explicit global/per-call guides still work.
updateSettings({ withGuide: false });

/** Wrap using visible columns rather than the length of ANSI escape sequences. */
export const wrapTextWithPrefix = (
  output: Writable | undefined,
  text: string,
  prefix: string,
  startPrefix = prefix,
): string => {
  const width = Math.max(
    1,
    getColumns(output ?? process.stdout) - Math.max(stringWidth(prefix), stringWidth(startPrefix)),
  );
  return wrapAnsi(text, width, { hard: true, trim: false })
    .split('\n')
    .map((line, index) => `${index === 0 ? startPrefix : prefix}${line}`)
    .join('\n');
};

export const getGuide = (opts?: CommonOptions): boolean => opts?.withGuide ?? settings.withGuide;

export const promptTitle = (message: string, state: State, opts: CommonOptions): string => {
  if (!message.trim()) {
    return '';
  }
  const hasGuide = getGuide(opts);
  return `${hasGuide ? `${color.gray(S_BAR)}\n` : ''}${wrapTextWithPrefix(
    opts.output,
    message,
    hasGuide ? `${symbolBar(state)} ` : '  ',
    `${symbol(state)} `,
  )}\n`;
};

export const optionText = (
  output: Writable | undefined,
  marker: string,
  label: string,
  format: (text: string) => string,
  suffix = '',
): string => {
  const continuation = ' '.repeat(stringWidth(marker));
  const width = Math.max(1, getColumns(output ?? process.stdout) - 2 - stringWidth(marker));
  const [first, ...rest] = label.split('\n');
  const content = [format(first) + suffix, ...rest.map(format)].join('\n');
  return wrapAnsi(content, width, { hard: true, trim: false })
    .split('\n')
    .map((line, index) => `${index === 0 ? marker : continuation}${line}`)
    .join('\n');
};

export const SELECT_INSTRUCTIONS = ['↑/↓ navigate', 'Enter confirm'];
export const MULTISELECT_INSTRUCTIONS = ['↑/↓ navigate', 'Space select', 'Enter confirm'];
export const formatInstructionFooter = (
  instructions: string[],
  hasGuide: boolean,
  output?: Writable,
): string[] =>
  wrapTextWithPrefix(
    output,
    color.dim(instructions.join(' · ')),
    hasGuide ? `${color.blue(S_BAR)} ` : '  ',
  ).split('\n');

export const unicode = isUnicodeSupported();
export const isCI = (): boolean => process.env.CI === 'true';
export const isTTY = (output: Writable): boolean => {
  return (output as Writable & { isTTY?: boolean }).isTTY === true;
};
export const unicodeOr = (c: string, fallback: string) => (unicode ? c : fallback);
export const S_POINTER_ACTIVE = unicodeOr('›', '>');
export const S_POINTER_INACTIVE = ' ';
export const S_STEP_ACTIVE = S_POINTER_ACTIVE;
export const S_STEP_CANCEL = unicodeOr('■', 'x');
export const S_STEP_ERROR = unicodeOr('▲', 'x');
export const S_STEP_SUBMIT = unicodeOr('◇', 'o');

export const S_BAR_START = unicodeOr('┌', 'T');
export const S_BAR = unicodeOr('│', '|');
export const S_BAR_END = unicodeOr('└', '—');
export const S_BAR_START_RIGHT = unicodeOr('┐', 'T');
export const S_BAR_END_RIGHT = unicodeOr('┘', '—');

export const S_RADIO_ACTIVE = S_POINTER_ACTIVE;
export const S_RADIO_INACTIVE = S_POINTER_INACTIVE;
export const S_CHECKBOX_ACTIVE = unicodeOr('◻', '[•]');
export const S_CHECKBOX_SELECTED = unicodeOr('◼', '[+]');
export const S_CHECKBOX_INACTIVE = unicodeOr('◻', '[ ]');
export const S_PASSWORD_MASK = unicodeOr('▪', '•');

export const S_BAR_H = unicodeOr('─', '-');
export const S_CORNER_TOP_RIGHT = unicodeOr('╮', '+');
export const S_CONNECT_LEFT = unicodeOr('├', '+');
export const S_CORNER_BOTTOM_RIGHT = unicodeOr('╯', '+');
export const S_CORNER_BOTTOM_LEFT = unicodeOr('╰', '+');
export const S_CORNER_TOP_LEFT = unicodeOr('╭', '+');

export const S_INFO = unicodeOr('●', '•');
export const S_SUCCESS = unicodeOr('◆', '*');
export const S_WARN = unicodeOr('▲', '!');
export const S_ERROR = unicodeOr('■', 'x');

export const completeColor = (value: string) => color.gray(value);

export const symbol = (state: State) => {
  switch (state) {
    case 'initial':
    case 'active':
      return color.blue(S_STEP_ACTIVE);
    case 'cancel':
      return color.red(S_STEP_CANCEL);
    case 'error':
      return color.yellow(S_STEP_ERROR);
    case 'validating':
      return color.dim(S_STEP_ACTIVE);
    case 'submit':
      return completeColor(S_STEP_SUBMIT);
    default:
      return color.blue(S_STEP_ACTIVE);
  }
};

export const symbolBar = (state: State) => {
  switch (state) {
    case 'initial':
    case 'active':
      return color.blue(S_BAR);
    case 'cancel':
      return color.red(S_BAR);
    case 'error':
      return color.yellow(S_BAR);
    case 'validating':
      return color.dim(S_BAR);
    case 'submit':
      return completeColor(S_BAR);
    default:
      return color.blue(S_BAR);
  }
};

export interface CommonOptions {
  input?: Readable;
  output?: Writable;
  signal?: AbortSignal;
  withGuide?: boolean;
  /** Static output for screen readers; also respects settings and ACCESSIBLE. */
  accessible?: boolean;
  /**
   * Stable identifier used in snapshot-test milestones
   * (`<kind>:<testId>:<state>`). Only read when `VP_EMIT_MILESTONES=1`;
   * defaults to the prompt kind. Set it when a flow shows several prompts of
   * the same kind, so tests can target each one unambiguously.
   */
  testId?: string;
}
