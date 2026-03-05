import type { Readable, Writable } from 'node:stream';

import type { State } from '@clack/core';
import isUnicodeSupported from 'is-unicode-supported';
import color from 'picocolors';

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
    case 'submit':
      return completeColor(S_STEP_SUBMIT);
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
    case 'submit':
      return completeColor(S_BAR);
  }
};

export interface CommonOptions {
  input?: Readable;
  output?: Writable;
  signal?: AbortSignal;
  withGuide?: boolean;
}
