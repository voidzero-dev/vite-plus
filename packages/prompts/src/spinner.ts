import readline from 'node:readline';
import type { Writable } from 'node:stream';

import { getColumns, isAccessible, settings } from '@clack/core';
import { wrapAnsi } from 'fast-wrap-ansi';
import color from 'picocolors';
import { cursor, erase } from 'sisteransi';

import {
  type CommonOptions,
  getGuide,
  isTTY,
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

const defaultStyleFn: SpinnerOptions['styleFrame'] = color.blue;

const removeTrailingDots = (msg: string): string => {
  return msg.replace(/\.+$/, '');
};

const formatTimer = (durationMs: number): string => {
  const duration = durationMs / 1000;
  const min = Math.floor(duration / 60);
  const secs = Math.floor(duration % 60);
  return color.gray(min > 0 ? `(${min}m ${secs}s)` : `(${secs}s)`);
};

const PROCESS_SIGNALS = ['SIGINT', 'SIGTERM'] as const;
type ProcessSignal = (typeof PROCESS_SIGNALS)[number];

// Signal listeners of active spinners. They do not count as another owner.
const spinnerSignalListeners = new Set<unknown>();

// Windows has no signals, so exit with the matching status there.
const endBySignal = (signal: ProcessSignal, pid: number): void => {
  if (process.platform === 'win32') {
    process.exit(signal === 'SIGINT' ? 130 : 143);
  } else {
    process.kill(pid, signal);
  }
};

// One spinner blocks input at a time, so one Ctrl+C is forwarded once.
let inputBlocked = false;

// clack's block() exits with 0 on Ctrl+C. This copy reports Ctrl+C and ignores other keys.
const blockInput = (output: Writable, onInterrupt: () => void): (() => void) => {
  if (inputBlocked) {
    return () => {};
  }
  inputBlocked = true;
  const input = process.stdin;
  const rl = readline.createInterface({ input, output, prompt: '', tabSize: 1 });
  readline.emitKeypressEvents(input, rl);
  if (input.isTTY) {
    input.setRawMode(true);
  }
  // One chunk can carry several keys, so the listener stays registered between them.
  const onKeypress = (_char: string | undefined, { name, sequence }: readline.Key): void => {
    if (sequence === '\x03') {
      onInterrupt();
      return;
    }
    // Erase the echoed key.
    const [dx, dy] = name === 'return' ? [0, -1] : [-1, 0];
    readline.moveCursor(output, dx, dy);
    readline.clearLine(output, 1);
  };
  output.write(cursor.hide);
  input.on('keypress', onKeypress);
  return () => {
    inputBlocked = false;
    input.off('keypress', onKeypress);
    output.write(cursor.show);
    // Windows keeps raw mode: https://github.com/bombshell-dev/clack/issues/176
    if (input.isTTY && process.platform !== 'win32') {
      input.setRawMode(false);
    }
    // @ts-expect-error https://github.com/nodejs/node/issues/31762#issuecomment-1441223907
    rl.terminal = false;
    rl.close();
  };
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
  const isCI = isCIFn() || !isTTY(output) || isAccessible(opts.accessible);

  let unblock: (() => void) | undefined;
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
  // A one-shot listener is removed before it runs, so keep a removal until this callback returns.
  const removedInDelivery = new Set<string | symbol>();
  const trackRemoval = (event: string | symbol, listener: unknown) => {
    if (!spinnerSignalListeners.has(listener)) {
      removedInDelivery.add(event);
      process.nextTick(() => removedInDelivery.delete(event));
    }
  };
  // Another listener of the signal decides how the process ends.
  const ownedByAnother = (signal: ProcessSignal) =>
    removedInDelivery.has(signal) ||
    process.listeners(signal).some((listener) => !spinnerSignalListeners.has(listener));
  // A listener replaces the default action, so the only owner raises the signal again.
  // An emit without the signal name only stops the spinner.
  const processSignalHandlers = PROCESS_SIGNALS.map((signal) => {
    const handler = (received?: NodeJS.Signals) => {
      const raise = received === signal && !ownedByAnother(signal);
      handleExit(1);
      if (raise) {
        endBySignal(signal, process.pid);
      }
    };
    return [signal, handler] as const;
  });
  // Raw mode turns Ctrl+C into a key, so send SIGINT to the process group as the terminal would.
  const interrupt = () => {
    handleExit(1);
    endBySignal('SIGINT', 0);
  };

  const registerHooks = () => {
    // Reference: https://nodejs.org/api/process.html#event-uncaughtexception
    process.on('uncaughtExceptionMonitor', errorEventHandler);
    // Reference: https://nodejs.org/api/process.html#event-unhandledrejection
    process.on('unhandledRejection', errorEventHandler);
    // Reference Signal Events: https://nodejs.org/api/process.html#signal-events
    for (const [signal, handler] of processSignalHandlers) {
      spinnerSignalListeners.add(handler);
      process.on(signal, handler);
    }
    process.on('exit', handleExit);
    process.on('removeListener', trackRemoval);

    if (signal) {
      signal.addEventListener('abort', signalEventHandler);
    }
  };

  const clearHooks = () => {
    process.removeListener('uncaughtExceptionMonitor', errorEventHandler);
    process.removeListener('unhandledRejection', errorEventHandler);
    process.removeListener('removeListener', trackRemoval);
    for (const [signal, handler] of processSignalHandlers) {
      process.removeListener(signal, handler);
      spinnerSignalListeners.delete(handler);
    }
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
      return;
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

  const hasGuide = getGuide(opts);

  const startLoop = (): void => {
    isSpinnerActive = true;
    unblock = isCI ? () => {} : blockInput(output, interrupt);
    _origin = performance.now();
    _prevMessage = undefined;
    if (hasGuide) {
      output.write(`${color.gray(S_BAR)}\n`);
    }
    let frameIndex = 0;
    let indicatorTimer = 0;
    registerHooks();
    const renderFrame = (): void => {
      if (isCI && _prevMessage !== undefined) {
        return;
      }
      clearPrevMessage();

      const frame = styleFn(frames[frameIndex] ?? (unicode ? '◒' : 'o'));
      let outputMessage: string;

      if (isCI) {
        outputMessage = `${frame} ${_message}...`;
      } else if (indicator === 'timer') {
        outputMessage = `${frame} ${_message} ${formatTimer(getElapsedMs())}`;
      } else {
        const loadingDots = '.'.repeat(Math.floor(indicatorTimer)).slice(0, 3);
        outputMessage = `${frame} ${_message}${loadingDots}`;
      }

      const wrapped = wrapAnsi(outputMessage, columns, {
        hard: true,
        trim: false,
      });
      output.write(isCI ? `${wrapped}\n` : wrapped);
      _prevMessage = wrapped;

      frameIndex = frameIndex + 1 < frames.length ? frameIndex + 1 : 0;
      // indicator increase by 1 every 8 frames
      indicatorTimer = indicatorTimer < 4 ? indicatorTimer + 0.125 : 0;
    };
    // Paint the first frame synchronously so the message shows the instant
    // start() is called, rather than only after the first interval tick. This
    // also means the message is visible before a synchronous, event-loop-blocking
    // operation that would otherwise starve the interval. The interval then
    // drives the ongoing animation.
    renderFrame();
    if (!isCI) {
      loop = setInterval(renderFrame, delay);
    }
    if (signal?.aborted) {
      signalEventHandler();
    }
  };

  const start = (msg = ''): void => {
    if (isSpinnerActive) {
      return;
    }
    isCancelled = false;
    _elapsedMs = 0;
    _message = removeTrailingDots(msg);
    startLoop();
  };

  // oxlint-disable-next-line no-underscore-dangle
  const _stop = (msg = '', code = 0, silent: boolean = false, preserveElapsed = false): void => {
    if (!isSpinnerActive) {
      return;
    }
    const elapsedMs = getElapsedMs();
    isSpinnerActive = false;
    isCancelled = code === 1;
    clearInterval(loop);
    clearPrevMessage();
    const step =
      code === 0
        ? completeColor(S_STEP_SUBMIT)
        : code === 1
          ? color.red(S_STEP_CANCEL)
          : color.red(S_STEP_ERROR);
    _message = msg ?? _message;
    if (!silent) {
      if (indicator === 'timer') {
        output.write(`${step} ${_message} ${formatTimer(elapsedMs)}\n`);
      } else {
        output.write(`${step} ${_message}\n`);
      }
    }
    if (!preserveElapsed) {
      _elapsedMs = 0;
    }
    _prevMessage = undefined;
    clearHooks();
    unblock?.();
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
