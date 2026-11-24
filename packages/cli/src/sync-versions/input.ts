import { Buffer } from 'node:buffer';

export const MAX_SYNC_VERSIONS_REQUEST_BYTES = 16 * 1024 * 1024;

export async function readBoundedUtf8(
  input: AsyncIterable<unknown>,
  maxBytes = MAX_SYNC_VERSIONS_REQUEST_BYTES,
): Promise<string> {
  const chunks: Array<Buffer> = [];
  let totalBytes = 0;

  for await (const chunk of input) {
    if (typeof chunk !== 'string' && !ArrayBuffer.isView(chunk)) {
      throw new TypeError('Expected UTF-8 input');
    }
    // Node also accepts non-byte ArrayBuffer views here. Its native conversion preserves their element semantics.
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- The guard above verifies the runtime overload accepted by Buffer.from.
    const buffer = Buffer.from(chunk as string | Uint8Array);
    totalBytes += buffer.byteLength;
    if (totalBytes > maxBytes) {
      throw new Error(`Sync request exceeds the ${maxBytes} byte limit`);
    }
    chunks.push(buffer);
  }

  return Buffer.concat(chunks, totalBytes).toString('utf8');
}
