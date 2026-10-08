import { gzipSync, gunzipSync } from 'node:zlib';

import { createTar } from 'nanotar';
import { pack, type Header } from 'tar-stream';
import { describe, expect, it, vi } from 'vitest';

import { ORG_TARBALL_LIMITS, readOrgTarball } from '../org-tarball-reader.ts';

async function archiveWith(
  files: { name: string; data?: string; type?: Header['type'] }[],
): Promise<Buffer> {
  const archive = pack();
  for (const { data = '', ...header } of files) {
    archive.entry(header, data);
  }
  archive.finalize();
  const chunks: Buffer[] = [];
  for await (const chunk of archive) {
    chunks.push(chunk as Buffer);
  }
  return gzipSync(Buffer.concat(chunks));
}

describe('readOrgTarball', () => {
  it('accepts entries exactly at the size, count, path and expanded-byte limits', async () => {
    const archive = await archiveWith([{ name: 'package/a', data: 'hello' }]);
    const contents: string[] = [];
    await readOrgTarball(
      archive,
      async (_header, data) => {
        for await (const chunk of data) {
          contents.push(chunk.toString());
        }
      },
      {
        ...ORG_TARBALL_LIMITS,
        expandedBytes: gunzipSync(archive).byteLength,
        entryBytes: 5,
        entries: 1,
        pathBytes: 9,
      },
    );
    expect(contents.join('')).toBe('hello');
  });

  it('limits all expanded bytes, including trailing tar padding', async () => {
    const archive = await archiveWith([{ name: 'package/a', data: 'hello' }]);
    const visit = vi.fn().mockResolvedValue(undefined);
    await expect(
      readOrgTarball(archive, visit, {
        ...ORG_TARBALL_LIMITS,
        expandedBytes: gunzipSync(archive).byteLength - 1,
      }),
    ).rejects.toThrow(/decompressed size limit/);
  });

  it('checks an advertised entry size before waiting for its missing body', async () => {
    const tar = Buffer.from(createTar([{ name: 'package/a', data: '' }]));
    tar.write('00000000100\0', 124, 'ascii'); // 64 bytes, with no body in this archive
    tar.fill(32, 148, 156);
    const checksum = tar.subarray(0, 512).reduce((sum, byte) => sum + byte, 0);
    tar.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 'ascii');
    const visit = vi.fn().mockResolvedValue(undefined);
    await expect(
      readOrgTarball(gzipSync(tar.subarray(0, 512)), visit, {
        ...ORG_TARBALL_LIMITS,
        entryBytes: 63,
      }),
    ).rejects.toThrow(/entry exceeds 63 byte size limit/);
    expect(visit).not.toHaveBeenCalled();
  });

  it('counts skipped files and directories before visiting the next entry', async () => {
    const archive = await archiveWith([
      { name: 'ignored/a', data: 'skipped' },
      { name: 'ignored/dir/', type: 'directory' },
    ]);
    const visit = vi.fn().mockResolvedValue(undefined);
    await expect(
      readOrgTarball(archive, visit, { ...ORG_TARBALL_LIMITS, entries: 1 }),
    ).rejects.toThrow(/entry count limit/);
    expect(visit).toHaveBeenCalledTimes(1);
  });

  it('applies the compression ratio limit after the allowance for small archives', async () => {
    const archive = await archiveWith([{ name: 'package/a', data: 'a'.repeat(16_384) }]);
    await expect(
      readOrgTarball(archive, async () => {}, {
        ...ORG_TARBALL_LIMITS,
        ratioAllowanceBytes: 1024,
        compressionRatio: 2,
      }),
    ).rejects.toThrow(/decompressed size limit/);
  });

  it('checks UTF-8 path bytes after resolving PAX extended names', async () => {
    const archive = await archiveWith([{ name: 'package/中文.txt', data: 'hello' }]);
    const visit = vi.fn().mockResolvedValue(undefined);
    await expect(
      readOrgTarball(archive, visit, { ...ORG_TARBALL_LIMITS, pathBytes: 17 }),
    ).rejects.toThrow(/path length limit/);
    expect(visit).not.toHaveBeenCalled();
  });

  it('preserves long paths encoded in PAX metadata', async () => {
    const name = `package/${'nested/'.repeat(30)}file.txt`;
    const archive = await archiveWith([{ name, data: 'hello' }]);
    const visit = vi.fn().mockResolvedValue(undefined);
    await readOrgTarball(archive, visit);
    expect(visit).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({ name }),
      expect.anything(),
    );
  });

  it('stops decompression when the visitor fails', async () => {
    const archive = await archiveWith([
      { name: 'package/a', data: 'a' },
      { name: 'package/b', data: 'b'.repeat(32_768) },
    ]);
    const visit = vi.fn().mockRejectedValue(new Error('write failed'));
    await expect(readOrgTarball(archive, visit)).rejects.toThrow('write failed');
    expect(visit).toHaveBeenCalledTimes(1);
  });

  it('rejects a truncated gzip stream', async () => {
    const archive = await archiveWith([{ name: 'package/a', data: 'hello' }]);
    await expect(readOrgTarball(archive.subarray(0, -8), async () => {})).rejects.toThrow();
  });
});
