import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  wrapTextWithPrefix,
  S_BAR,
  S_ERROR,
  S_INFO,
  S_STEP_SUBMIT,
  S_SUCCESS,
  S_WARN,
  completeColor,
} from './common.js';

export interface LogMessageOptions extends CommonOptions {
  symbol?: string;
  spacing?: number;
  secondarySymbol?: string;
}

export const log = {
  message: (
    message: string | string[] = [],
    {
      symbol,
      secondarySymbol = color.gray(S_BAR),
      output = process.stdout,
      spacing = 0,
      withGuide,
    }: LogMessageOptions = {},
  ) => {
    const parts: string[] = [];
    const hasGuide = getGuide({ withGuide });
    const spacingString = !hasGuide ? '' : secondarySymbol;
    const prefix = symbol ? `${symbol} ` : hasGuide ? `${secondarySymbol} ` : '';
    const marker = symbol ?? secondarySymbol;
    const secondaryPrefix = hasGuide ? `${secondarySymbol} ` : symbol ? '  ' : '';

    for (let i = 0; i < spacing; i++) {
      parts.push(spacingString);
    }

    const messageParts = Array.isArray(message) ? message : message.split('\n');
    if (messageParts.length > 0) {
      const [firstLine, ...lines] = messageParts;
      if (firstLine.length > 0) {
        parts.push(wrapTextWithPrefix(output, firstLine, secondaryPrefix, prefix));
      } else {
        parts.push(hasGuide || symbol ? marker : '');
      }
      for (const ln of lines) {
        if (ln.length > 0) {
          parts.push(wrapTextWithPrefix(output, ln, secondaryPrefix));
        } else {
          parts.push(hasGuide ? secondarySymbol : '');
        }
      }
    }
    output.write(`${parts.join('\n')}\n`);
  },
  info: (message: string, opts?: LogMessageOptions) => {
    log.message(message, { ...opts, symbol: color.blue(S_INFO) });
  },
  success: (message: string, opts?: LogMessageOptions) => {
    log.message(message, { ...opts, symbol: completeColor(S_SUCCESS) });
  },
  step: (message: string, opts?: LogMessageOptions) => {
    log.message(message, { ...opts, symbol: completeColor(S_STEP_SUBMIT) });
  },
  warn: (message: string, opts?: LogMessageOptions) => {
    log.message(message, { ...opts, symbol: color.yellow(S_WARN) });
  },
  /** alias for `log.warn()`. */
  warning: (message: string, opts?: LogMessageOptions) => {
    log.warn(message, opts);
  },
  error: (message: string, opts?: LogMessageOptions) => {
    log.message(message, { ...opts, symbol: color.red(S_ERROR) });
  },
};
