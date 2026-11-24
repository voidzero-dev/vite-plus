import type { CANCEL_SYMBOL, Prompt } from '@clack/core';

export type PromptResult<Value> = Promise<Value | typeof CANCEL_SYMBOL>;

export function runPrompt<Value>(prompt: Pick<Prompt<Value>, 'prompt'>): PromptResult<Value> {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Keep the public result type while forwarding Clack's optional value unchanged for an empty selection.
  return prompt.prompt() as PromptResult<Value>;
}
