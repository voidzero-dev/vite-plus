import { promisify } from 'node:util';
import { gunzip } from 'node:zlib';

import { parseTar, type ParsedTarFileItem } from 'nanotar';

const decompress = promisify(gunzip);
const TAR_BLOCK_BYTES = 512;

export const ORG_TARBALL_LIMITS = {
  expandedBytes: 100 * 1024 * 1024,
  entryBytes: 20 * 1024 * 1024,
  entries: 10_000,
  pathBytes: 4096,
  metadataBytes: 1024 * 1024,
  // nanotar copies global PAX attributes into every subsequent entry.
  globalMetadataFields: 128,
  compressionRatio: 200,
  // Small archives have proportionally more tar padding and repeated headers.
  ratioAllowanceBytes: 1024 * 1024,
};

/** Check raw records, including metadata that nanotar processes before its filter. */
function validateTarRecords(tar: Buffer, limits: typeof ORG_TARBALL_LIMITS): void {
  let offset = 0;
  let entries = 0;
  let metadataBytes = 0;
  let globalMetadataFields = 0;
  while (offset < tar.byteLength) {
    if (offset + TAR_BLOCK_BYTES > tar.byteLength) {
      throw new Error('truncated tarball header');
    }
    // nanotar stops at the first header with an empty name.
    if (tar[offset] === 0) {
      return;
    }
    entries++;
    if (entries > limits.entries) {
      throw new Error(`tarball exceeds ${limits.entries} entry count limit`);
    }
    const rawSize = tar
      .toString('latin1', offset + 124, offset + 136)
      .split('\0', 1)[0]
      .trim();
    const size = Number.parseInt(rawSize, 8);
    // Match nanotar's octal sizes, but reject values that could stall or reverse its cursor.
    if (!/^[0-7]+$/.test(rawSize) || !Number.isSafeInteger(size)) {
      throw new Error('tarball entry has an invalid size');
    }
    if (size > limits.entryBytes) {
      throw new Error(`tarball entry exceeds ${limits.entryBytes} byte size limit`);
    }
    const dataOffset = offset + TAR_BLOCK_BYTES;
    const nextOffset = dataOffset + Math.ceil(size / TAR_BLOCK_BYTES) * TAR_BLOCK_BYTES;
    if (nextOffset > tar.byteLength) {
      throw new Error('truncated tarball entry');
    }
    const type = String.fromCharCode(tar[offset + 156]);
    if (['x', 'g', 'L', 'K', 'N'].includes(type)) {
      metadataBytes += size;
      if (metadataBytes > limits.metadataBytes) {
        throw new Error(`tarball exceeds ${limits.metadataBytes} byte metadata size limit`);
      }
    }
    if (type === 'g') {
      // nanotar splits PAX fields on newlines. Count before it allocates or merges them.
      globalMetadataFields++;
      for (const byte of tar.subarray(dataOffset, dataOffset + size)) {
        if (byte === 10) {
          globalMetadataFields++;
        }
      }
      if (globalMetadataFields > limits.globalMetadataFields) {
        throw new Error(
          `tarball exceeds ${limits.globalMetadataFields} global metadata field limit`,
        );
      }
    }
    offset = nextOffset;
  }
}

/** Bound decompression and parser allocations before exposing entries to callers. */
export async function parseOrgTarball(
  bytes: Uint8Array,
  limits = ORG_TARBALL_LIMITS,
): Promise<ParsedTarFileItem[]> {
  const expandedLimit = Math.min(
    limits.expandedBytes,
    Math.max(limits.ratioAllowanceBytes, bytes.byteLength * limits.compressionRatio),
  );
  let tar: Buffer;
  try {
    // Node stops decompression as soon as its output exceeds maxOutputLength.
    tar = await decompress(bytes, { maxOutputLength: expandedLimit });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ERR_BUFFER_TOO_LARGE') {
      throw new Error(`tarball exceeds ${expandedLimit} byte decompressed size limit`, {
        cause: error,
      });
    }
    throw error;
  }
  validateTarRecords(tar, limits);
  // nanotar reads the entire backing ArrayBuffer, ignoring a Buffer's byteOffset.
  const data = new Uint8Array(tar);
  return parseTar(data, {
    filter(entry) {
      const attrs = entry.attrs as Record<string, unknown> | undefined;
      // Also check stored extended names, before path normalization can shorten them.
      for (const name of [entry.name, attrs?.path, attrs?.linkpath]) {
        if (typeof name === 'string' && Buffer.byteLength(name) > limits.pathBytes) {
          throw new Error(`tarball entry exceeds ${limits.pathBytes} byte path length limit`);
        }
      }
      return true;
    },
  });
}
