import { block, getColumns, settings } from '@clack/core';
import { wrapAnsi } from 'fast-wrap-ansi';
import color from 'picocolors';
import { cursor, erase } from 'sisteransi';

import {
  type CommonOptions,
  completeColor,
  isCI as isCIFn,
  S_BAR,
  S_STEP_CANCEL,
  S_STEP_ERROR,
  S_STEP_SUBMIT,
  unicode,
} from './common.js';

export interface SpinnerOptions extends CommonOptions {
  indicator?: 'dots' | 'timer';
  onCancel?: () => void;
  cancelMessage?: string;
  errorMessage?: string;
  frames?: string[];
  delay?: number;
  styleFrame?: (frame: string) => string;
}

export interface SpinnerResult {
  start(msg?: string): void;
  pause(): void;
  resume(msg?: string): void;
  stop(msg?: string): void;
  cancel(msg?: string): void;
  error(msg?: string): void;
  message(msg?: string): void;
  clear(): void;
  readonly isCancelled: boolean;
}

const defaultStyleFn: SpinnerOptions['styleFrame'] = color.magenta;

const removeTrailingDots = (msg: string): string => {
  return msg.replace(/\.+$/, '');
};

const formatTimer = (durationMs: number): string => {
  const duration = durationMs / 1000;
  const min = Math.floor(duration / 60);
  const secs = Math.floor(duration % 60);
  return color.gray(min > 0 ? `(${min}m ${secs}s)` : `(${secs}s)`);
};

export const spinner = ({
  indicator = 'dots',
  onCancel,
  output = process.stdout,
  cancelMessage,
  errorMessage,
  frames = unicode ? ['◒', '◐', '◓', '◑'] : ['•', 'o', 'O', '0'],
  delay = unicode ? 80 : 120,
  signal,
  ...opts
}: SpinnerOptions = {}): SpinnerResult => {
  const isCI = isCIFn();

  let unblock: () => void;
  let loop: NodeJS.Timeout;
  let isSpinnerActive = false;
  let isCancelled = false;
  /* oxlint-disable no-underscore-dangle */
  let _message = '';
  let _prevMessage: string | undefined;
  let _origin: number = performance.now();
  let _elapsedMs = 0;
  /* oxlint-enable no-underscore-dangle */
  const columns = getColumns(output);
  const styleFn = opts?.styleFrame ?? defaultStyleFn;

  const getElapsedMs = () => {
    if (!isSpinnerActive) {
      return _elapsedMs;
    }
    return _elapsedMs + (performance.now() - _origin);
  };

  const handleExit = (code: number) => {
    const msg =
      code > 1
        ? (errorMessage ?? settings.messages.error)
        : (cancelMessage ?? settings.messages.cancel);
    isCancelled = code === 1;
    if (isSpinnerActive) {
      _stop(msg, code);
      if (isCancelled && typeof onCancel === 'function') {
        onCancel();
      }
    }
  };

  const errorEventHandler = () => handleExit(2);
  const signalEventHandler = () => handleExit(1);

  const registerHooks = () => {
    // Reference: https://nodejs.org/api/process.html#event-uncaughtexception
    process.on('uncaughtExceptionMonitor', errorEventHandler);
    // Reference: https://nodejs.org/api/process.html#event-unhandledrejection
    process.on('unhandledRejection', errorEventHandler);
    // Reference Signal Events: https://nodejs.org/api/process.html#signal-events
    process.on('SIGINT', signalEventHandler);
    process.on('SIGTERM', signalEventHandler);
    process.on('exit', handleExit);

    if (signal) {
      signal.addEventListener('abort', signalEventHandler);
    }
  };

  const clearHooks = () => {
    process.removeListener('uncaughtExceptionMonitor', errorEventHandler);
    process.removeListener('unhandledRejection', errorEventHandler);
    process.removeListener('SIGINT', signalEventHandler);
    process.removeListener('SIGTERM', signalEventHandler);
    process.removeListener('exit', handleExit);

    if (signal) {
      signal.removeEventListener('abort', signalEventHandler);
    }
  };

  const clearPrevMessage = () => {
    if (_prevMessage === undefined) {
      return;
    }
    if (isCI) {
      output.write('\n');
    }
    const wrapped = wrapAnsi(_prevMessage, columns, {
      hard: true,
      trim: false,
    });
    const prevLines = wrapped.split('\n');
    if (prevLines.length > 1) {
      output.write(cursor.up(prevLines.length - 1));
    }
    output.write(cursor.to(0));
    output.write(erase.down());
  };

  const hasGuide = opts.withGuide ?? false;

  const startLoop = (): void => {
    isSpinnerActive = true;
    unblock = block({ output });
    _origin = performance.now();
    _prevMessage = undefined;
    if (hasGuide) {
      output.write(`${color.gray(S_BAR)}\n`);
    }
    let frameIndex = 0;
    let indicatorTimer = 0;
    registerHooks();
    loop = setInterval(() => {
      if (isCI && _message === _prevMessage) {
        return;
      }
      clearPrevMessage();
      _prevMessage = _message;
      const frame = styleFn(frames[frameIndex]);
      let outputMessage: string;

      if (isCI) {
        outputMessage = `${frame}  ${_message}...`;
      } else if (indicator === 'timer') {
        outputMessage = `${frame}  ${_message} ${formatTimer(getElapsedMs())}`;
      } else {
        const loadingDots = '.'.repeat(Math.floor(indicatorTimer)).slice(0, 3);
        outputMessage = `${frame}  ${_message}${loadingDots}`;
      }

      const wrapped = wrapAnsi(outputMessage, columns, {
        hard: true,
        trim: false,
      });
      output.write(wrapped);

      frameIndex = frameIndex + 1 < frames.length ? frameIndex + 1 : 0;
      // indicator increase by 1 every 8 frames
      indicatorTimer = indicatorTimer < 4 ? indicatorTimer + 0.125 : 0;
    }, delay);
  };

  const start = (msg = ''): void => {
    _elapsedMs = 0;
    _message = removeTrailingDots(msg);
    startLoop();
  };

  // oxlint-disable-next-line no-underscore-dangle
  const _stop = (msg = '', code = 0, silent: boolean = false, preserveElapsed = false): void => {
    if (!isSpinnerActive) {
      return;
    }
    isSpinnerActive = false;
    clearInterval(loop);
    clearPrevMessage();
    const elapsedMs = getElapsedMs();
    const step =
      code === 0
        ? completeColor(S_STEP_SUBMIT)
        : code === 1
          ? color.red(S_STEP_CANCEL)
          : color.red(S_STEP_ERROR);
    _message = msg ?? _message;
    if (!silent) {
      if (indicator === 'timer') {
        output.write(`${step} ${_message} ${formatTimer(elapsedMs)}\n\n`);
      } else {
        output.write(`${step} ${_message}\n\n`);
      }
    }
    if (!preserveElapsed) {
      _elapsedMs = 0;
    }
    _prevMessage = undefined;
    clearHooks();
    unblock();
  };

  const pause = (): void => {
    if (!isSpinnerActive) {
      return;
    }
    _elapsedMs = getElapsedMs();
    _stop(_message, 0, true, true);
  };
  const resume = (msg = _message): void => {
    if (isSpinnerActive) {
      return;
    }
    _message = removeTrailingDots(msg);
    startLoop();
  };
  const stop = (msg = ''): void => _stop(msg, 0);
  const cancel = (msg = ''): void => _stop(msg, 1);
  const error = (msg = ''): void => _stop(msg, 2);
  const clear = (): void => _stop('', 0, true);

  const message = (msg = ''): void => {
    _message = removeTrailingDots(msg ?? _message);
  };

  return {
    start,
    pause,
    resume,
    stop,
    message,
    cancel,
    error,
    clear,
    get isCancelled() {
      return isCancelled;
    },
  };
};
