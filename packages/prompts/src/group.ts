import { isCancel } from '@clack/core';

type Prettify<T> = {
  [P in keyof T]: T[P];
} & {};

export type PromptGroupAwaitedReturn<T> = {
  [P in keyof T]: Exclude<Awaited<T[P]>, symbol>;
};

export interface PromptGroupOptions<T> {
  /**
   * Control how the group can be canceled
   * if one of the prompts is canceled.
   */
  onCancel?: (opts: { results: Prettify<Partial<PromptGroupAwaitedReturn<T>>> }) => void;
}

export type PromptGroup<T> = {
  [P in keyof T]: (opts: {
    results: Prettify<Partial<PromptGroupAwaitedReturn<Omit<T, P>>>>;
  }) => undefined | Promise<T[P] | undefined>;
};

/**
 * Define a group of prompts to be displayed
 * and return a results of objects within the group
 */
export const group = async <T>(
  prompts: PromptGroup<T>,
  opts?: PromptGroupOptions<T>,
): Promise<Prettify<PromptGroupAwaitedReturn<T>>> => {
  const results: Partial<PromptGroupAwaitedReturn<T>> = {};

  for (const name in prompts) {
    if (!Object.hasOwn(prompts, name)) {
      continue;
    }
    const prompt = prompts[name];
    const result = await prompt({ results });

    // Pass the results to the onCancel function
    // so the user can decide what to do with the results
    // TODO: Switch to callback within core to avoid isCancel Fn
    if (typeof opts?.onCancel === 'function' && isCancel(result)) {
      Object.assign(results, { [name]: 'canceled' });
      opts.onCancel({ results });
      continue;
    }

    Object.assign(results, { [name]: result });
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Every prompt has now populated its corresponding result.
  return results as PromptGroupAwaitedReturn<T>;
};
