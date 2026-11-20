import { randomUUID } from 'node:crypto';
import { appendFileSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

export interface CommandTimeout {
  command: string;
  description: string | undefined;
  timeoutMs: number;
}

type ContentBlock = {
  type?: unknown;
  id?: unknown;
  tool_use_id?: unknown;
  input?: { command?: unknown; description?: unknown };
};

type SdkMessage = {
  type?: unknown;
  message?: { content?: unknown };
  tool_use_result?: { timedOutAfterMs?: unknown };
};

/**
 * Finds Bash calls that exceeded Claude Code's per-command timeout in the SDK
 * messages that claude-code-action writes to its `execution_file` output.
 */
export function findCommandTimeouts(messages: unknown): CommandTimeout[] {
  if (!Array.isArray(messages)) {
    throw new Error('Expected the execution file to contain an array of SDK messages');
  }
  const inputs = new Map<unknown, ContentBlock['input']>();
  const timeouts: CommandTimeout[] = [];
  for (const message of messages as SdkMessage[]) {
    const content = message?.message?.content;
    if (!Array.isArray(content)) {
      continue;
    }
    const blocks = content as ContentBlock[];
    if (message.type === 'assistant') {
      for (const block of blocks) {
        if (block?.type === 'tool_use') {
          inputs.set(block.id, block.input);
        }
      }
      continue;
    }
    // Each tool result is sent in its own user message with the structured result.
    const timeoutMs = message.tool_use_result?.timedOutAfterMs;
    const result = blocks.find((block) => block?.type === 'tool_result');
    if (message.type !== 'user' || typeof timeoutMs !== 'number' || !result) {
      continue;
    }
    const input = inputs.get(result.tool_use_id);
    timeouts.push({
      command: typeof input?.command === 'string' ? input.command : '',
      description: typeof input?.description === 'string' ? input.description : undefined,
      timeoutMs,
    });
  }
  return timeouts;
}

/** Formats the timeouts as a Markdown list with each command in a code block. */
export function formatCommandTimeouts(timeouts: CommandTimeout[]): string {
  return timeouts
    .map(({ command, description, timeoutMs }) => {
      const longestBackticks = Math.max(
        0,
        ...Array.from(command.matchAll(/`+/g), (m) => m[0].length),
      );
      const fence = '`'.repeat(Math.max(3, longestBackticks + 1));
      return [
        `- ${description ?? 'Command'} (timed out after ${timeoutMs / 1000}s)`,
        `  ${fence}sh`,
        ...command.split('\n').map((line) => `  ${line}`),
        `  ${fence}`,
      ].join('\n');
    })
    .join('\n');
}

const invokedPath = process.argv[1];
if (invokedPath && pathToFileURL(resolve(invokedPath)).href === import.meta.url) {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    options: { 'github-output': { type: 'string' } },
  });
  const [executionFile] = positionals;
  // The action writes no execution file when its step times out or is cancelled.
  const report =
    executionFile && existsSync(executionFile)
      ? formatCommandTimeouts(findCommandTimeouts(JSON.parse(readFileSync(executionFile, 'utf8'))))
      : '';
  console.log(report || 'No command timeouts found.');
  if (values['github-output']) {
    // Commands come from the agent, so use a delimiter it cannot predict.
    const delimiter = `EOF_${randomUUID()}`;
    appendFileSync(values['github-output'], `report<<${delimiter}\n${report}\n${delimiter}\n`);
  }
}
