import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { createGunzip } from 'node:zlib';

import { extract, type Header } from 'tar-stream';

export const ORG_TARBALL_LIMITS = {
  expandedBytes: 100 * 1024 * 1024,
  entryBytes: 20 * 1024 * 1024,
  entries: 10_000,
  pathBytes: 4096,
  compressionRatio: 200,
  // Small archives have proportionally more tar padding and repeated headers.
  ratioAllowanceBytes: 1024 * 1024,
};

/** Visit entries sequentially, without buffering the expanded archive. */
export async function readOrgTarball(
  bytes: Uint8Array,
  visit: (header: Header, data: AsyncIterable<Buffer>) => Promise<void>,
  limits = ORG_TARBALL_LIMITS,
): Promise<void> {
  const parser = extract();
  const expandedLimit = Math.min(
    limits.expandedBytes,
    Math.max(limits.ratioAllowanceBytes, bytes.byteLength * limits.compressionRatio),
  );
  const transfer = pipeline(
    Readable.from([bytes]),
    createGunzip(),
    async function* (source: AsyncIterable<Buffer>) {
      let expanded = 0;
      for await (const chunk of source) {
        expanded += chunk.byteLength;
        if (expanded > expandedLimit) {
          throw new Error(`tarball exceeds ${expandedLimit} byte decompressed size limit`);
        }
        yield chunk;
      }
    },
    parser,
  );
  const consume = (async () => {
    let entries = 0;
    for await (const entry of parser) {
      if (++entries > limits.entries) {
        throw new Error(`tarball exceeds ${limits.entries} entry count limit`);
      }
      const { header } = entry;
      if (!Number.isSafeInteger(header.size) || header.size < 0) {
        throw new Error('tarball entry has an invalid size');
      }
      if (header.size > limits.entryBytes) {
        throw new Error(`tarball entry exceeds ${limits.entryBytes} byte size limit`);
      }
      if (Buffer.byteLength(header.name) > limits.pathBytes) {
        throw new Error(`tarball entry exceeds ${limits.pathBytes} byte path length limit`);
      }
      // tar-stream emits binary chunks; streamx types its iterator values as unknown.
      await visit(header, entry as AsyncIterable<Buffer>);
      // A visitor can skip an entry, but it still counts towards every limit.
      entry.resume();
    }
  })();
  try {
    await Promise.all([transfer, consume]);
  } catch (error) {
    parser.destroy(error as Error);
    // Wait for the active visitor to stop before extraction removes its staging tree.
    await Promise.allSettled([transfer, consume]);
    throw error;
  }
}
