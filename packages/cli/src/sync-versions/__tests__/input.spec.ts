import { Readable } from 'node:stream';

import { describe, expect, it } from 'vitest';

import { readBoundedUtf8 } from '../input.ts';

async function* invalidInput(): AsyncGenerator<unknown> {
  yield 42;
}

describe('readBoundedUtf8', () => {
  it('reads chunked UTF-8 input without changing it', async () => {
    const input = Readable.from(['{"schema', 'Version":1}\n']);

    await expect(readBoundedUtf8(input, 64)).resolves.toBe('{"schemaVersion":1}\n');
  });

  it('rejects input larger than the byte limit', async () => {
    const input = Readable.from(['1234', '5678']);

    await expect(readBoundedUtf8(input, 7)).rejects.toThrow('exceeds the 7 byte limit');
  });

  it('measures bytes rather than JavaScript string length', async () => {
    const input = Readable.from(['é']);

    await expect(readBoundedUtf8(input, 1)).rejects.toThrow('exceeds the 1 byte limit');
  });

  it('rejects non-byte input chunks', async () => {
    await expect(readBoundedUtf8(invalidInput())).rejects.toThrow('Expected UTF-8 input');
  });
});
