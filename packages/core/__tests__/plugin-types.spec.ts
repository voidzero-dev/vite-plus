import type { Plugin as RolldownPlugin } from '@voidzero-dev/vite-plus-core/rolldown';
import type { Plugin } from 'vite-plus';
import { assertType, test } from 'vite-plus/test';

type LoadHook = Exclude<Plugin['load'], { handler: unknown } | undefined>;
type LoadOptions = Parameters<LoadHook>[1];

test('Vite load hooks accept Rolldown options and SSR-only calls', () => {
  const plugin: Plugin = {
    name: 'load-options-compatibility',
    load(_id, options) {
      assertType<string | undefined>(options?.rawId);
      assertType<Record<string, string> | undefined>(options?.attributes);
      assertType<boolean | undefined>(options?.ssr);
      return null;
    },
  };

  assertType<RolldownPlugin>(plugin);
  assertType<LoadOptions>(undefined);
  assertType<LoadOptions>({ ssr: true });
  assertType<LoadOptions>({
    rawId: '/data.json',
    attributes: { type: 'json' },
    ssr: false,
  });
});
