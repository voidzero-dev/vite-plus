import { describe, expect, it } from 'vite-plus/test';

import cliPkg from '../../cli/package.json' with { type: 'json' };
import { assertCoreVersionMatchesVite } from '../../tools/src/core-version.ts';
import corePkg from '../package.json' with { type: 'json' };

describe('core version', () => {
  // The preview flow stamps `0.0.0-commit.<sha>` on every package.
  it.skipIf(corePkg.version.startsWith('0.0.0-commit.'))('encodes the bundled Vite version', () => {
    expect(() => assertCoreVersionMatchesVite(corePkg, cliPkg.version)).not.toThrow();
  });
});
