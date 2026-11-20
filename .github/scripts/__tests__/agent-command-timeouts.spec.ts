/// <reference types="node" />

import { expect, test } from 'vite-plus/test';

import { findCommandTimeouts, formatCommandTimeouts } from '../agent-command-timeouts.ts';

function toolUse(id: string, command: string, description?: string) {
  return {
    type: 'assistant',
    message: {
      role: 'assistant',
      content: [{ type: 'tool_use', id, name: 'Bash', input: { command, description } }],
    },
  };
}

function toolResult(id: string, toolUseResult: Record<string, unknown>) {
  return {
    type: 'user',
    message: {
      role: 'user',
      content: [{ tool_use_id: id, type: 'tool_result', content: '', is_error: false }],
    },
    tool_use_result: toolUseResult,
  };
}

test('finds Bash calls that exceeded their timeout', () => {
  const messages = [
    { type: 'system', subtype: 'init' },
    toolUse('fast', 'just build', 'Build'),
    toolResult('fast', { stdout: 'ok', stderr: '', interrupted: false }),
    toolUse('slow', 'pnpm build-native --release', 'Rebuild binding'),
    { type: 'tool_progress', tool_use_id: 'slow-heartbeat-1', elapsed_time_seconds: 600 },
    toolResult('slow', { backgroundTaskId: 'b1', timedOutAfterMs: 600000 }),
    { type: 'result', is_error: true },
  ];

  expect(findCommandTimeouts(messages)).toEqual([
    { command: 'pnpm build-native --release', description: 'Rebuild binding', timeoutMs: 600000 },
  ]);
});

test('keeps a timeout whose call is missing from the messages', () => {
  expect(findCommandTimeouts([toolResult('unknown', { timedOutAfterMs: 120000 })])).toEqual([
    { command: '', description: undefined, timeoutMs: 120000 },
  ]);
});

test('rejects an execution file without a message array', () => {
  expect(() => findCommandTimeouts({ messages: [] })).toThrow('array of SDK messages');
});

test('formats each command in a code block inside the list item', () => {
  expect(
    formatCommandTimeouts([
      { command: 'cargo check\ncargo shear', description: 'Check Rust', timeoutMs: 600000 },
      { command: 'echo ```', description: undefined, timeoutMs: 120000 },
    ]),
  ).toBe(
    [
      '- Check Rust (timed out after 600s)',
      '  ```sh',
      '  cargo check',
      '  cargo shear',
      '  ```',
      '- Command (timed out after 120s)',
      '  ````sh',
      '  echo ```',
      '  ````',
    ].join('\n'),
  );
});
