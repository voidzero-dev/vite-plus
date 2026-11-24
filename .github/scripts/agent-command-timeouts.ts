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

interface SdkObject {
  type?: unknown;
  message?: unknown;
  content?: unknown;
  id?: unknown;
  input?: unknown;
  command?: unknown;
  description?: unknown;
  tool_use_result?: unknown;
  timedOutAfterMs?: unknown;
  tool_use_id?: unknown;
}

function isSdkObject(value: unknown): value is SdkObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Finds Bash calls that exceeded Claude Code's per-command timeout in the SDK
 * messages that claude-code-action writes to its `execution_file` output.
 */
export function findCommandTimeouts(messages: unknown): Array<CommandTimeout> {
  if (!Array.isArray(messages)) {
    throw new Error('Expected the execution file to contain an array of SDK messages');
  }
  const inputs = new Map<string, { command: string; description: string | undefined }>();
  const timeouts: Array<CommandTimeout> = [];
  const entries: Array<unknown> = messages;
  for (const entry of entries) {
    if (
      !isSdkObject(entry) ||
      !isSdkObject(entry.message) ||
      !Array.isArray(entry.message.content)
    ) {
      continue;
    }
    const blocks = entry.message.content.filter(isSdkObject);
    if (entry.type === 'assistant') {
      for (const block of blocks) {
        if (block.type === 'tool_use' && typeof block.id === 'string' && isSdkObject(block.input)) {
          const { command, description } = block.input;
          inputs.set(block.id, {
            command: typeof command === 'string' ? command : '',
            description: typeof description === 'string' ? description : undefined,
          });
        }
      }
      continue;
    }
    const details = entry.tool_use_result;
    if (
      entry.type !== 'user' ||
      !isSdkObject(details) ||
      typeof details.timedOutAfterMs !== 'number'
    ) {
      continue;
    }
    const result = blocks.find((block) => block.type === 'tool_result');
    if (result) {
      const input =
        typeof result.tool_use_id === 'string' ? inputs.get(result.tool_use_id) : undefined;
      timeouts.push({
        command: input?.command ?? '',
        description: input?.description,
        timeoutMs: details.timedOutAfterMs,
      });
    }
  }
  return timeouts;
}

/** Formats the timeouts as a Markdown list with each command in a code block. */
export function formatCommandTimeouts(timeouts: Array<CommandTimeout>): string {
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
