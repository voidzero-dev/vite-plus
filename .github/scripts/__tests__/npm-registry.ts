import type { FetchLike } from '../wait-for-npm-packages.ts';

export function response(status: number, body?: unknown): Awaited<ReturnType<FetchLike>> {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  };
}

export function stalledFetch(_url: string, init?: Parameters<FetchLike>[1]): ReturnType<FetchLike> {
  return new Promise((_resolve, reject) => {
    const signal = init?.signal;
    if (!signal) {
      reject(new Error('missing abort signal'));
      return;
    }
    signal.throwIfAborted();
    signal.addEventListener('abort', () => reject(signal.reason), { once: true });
  });
}
