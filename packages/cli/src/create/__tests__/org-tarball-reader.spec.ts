import { gzipSync, gunzipSync } from 'node:zlib';

import { createTar } from 'nanotar';
import { describe, expect, it } from 'vitest';

import { ORG_TARBALL_LIMITS, parseOrgTarball } from '../org-tarball-reader.ts';

type TestEntry = { name: string; data?: string; type?: string };

function tarRecord({ name, data = '', type = '0' }: TestEntry): Buffer {
  const tar = Buffer.from(createTar([{ name, data }]));
  tar[156] = type.charCodeAt(0);
  tar.fill(32, 148, 156);
  const checksum = tar.subarray(0, 512).reduce((sum, byte) => sum + byte, 0);
  tar.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 'ascii');
  return tar.subarray(0, 512 + Math.ceil(Buffer.byteLength(data) / 512) * 512);
}

function archiveWith(files: TestEntry[]): Buffer {
  return gzipSync(Buffer.concat([...files.map(tarRecord), Buffer.alloc(1024)]));
}

function paxField(key: string, value: string): string {
  const field = ` ${key}=${value}\n`;
  const fieldBytes = Buffer.byteLength(field);
  let length = fieldBytes + 1;
  while (String(length).length + fieldBytes !== length) {
    length = String(length).length + fieldBytes;
  }
  return `${length}${field}`;
}

describe('parseOrgTarball', () => {
  it('accepts entries exactly at the size, count, path and expanded-byte limits', async () => {
    const archive = archiveWith([{ name: 'package/a', data: 'hello' }]);
    const entries = await parseOrgTarball(archive, {
      ...ORG_TARBALL_LIMITS,
      expandedBytes: gunzipSync(archive).byteLength,
      entryBytes: 5,
      entries: 1,
      pathBytes: 9,
    });
    expect(entries).toHaveLength(1);
    expect(entries[0].text).toBe('hello');
  });

  it('limits all expanded bytes, including trailing tar padding', async () => {
    const archive = archiveWith([{ name: 'package/a', data: 'hello' }]);
    await expect(
      parseOrgTarball(archive, {
        ...ORG_TARBALL_LIMITS,
        expandedBytes: gunzipSync(archive).byteLength - 1,
      }),
    ).rejects.toThrow(/decompressed size limit/);
  });

  it('checks the advertised size before reading a missing entry body', async () => {
    const tar = tarRecord({ name: 'package/a' });
    tar.write('00000000100\0', 124, 'ascii'); // 64 bytes, with no body in this archive
    await expect(
      parseOrgTarball(gzipSync(tar), { ...ORG_TARBALL_LIMITS, entryBytes: 63 }),
    ).rejects.toThrow(/entry exceeds 63 byte size limit/);
  });

  it.each(['-0000000001', 'not-octal', '1000\xb0'])(
    'rejects a malformed size field: %s',
    async (size) => {
      const tar = tarRecord({ name: 'package/a' });
      tar.fill(0, 124, 136);
      tar.write(size, 124, 'latin1');
      await expect(parseOrgTarball(gzipSync(tar))).rejects.toThrow(/invalid size/);
    },
  );

  it('rejects a truncated entry body', async () => {
    const tar = tarRecord({ name: 'package/a', data: 'hello' });
    await expect(parseOrgTarball(gzipSync(tar.subarray(0, -1)))).rejects.toThrow(
      /truncated tarball entry/,
    );
  });

  it('rejects a truncated header', async () => {
    const tar = tarRecord({ name: 'package/a' });
    await expect(parseOrgTarball(gzipSync(tar.subarray(0, 100)))).rejects.toThrow(
      /truncated tarball header/,
    );
  });

  it('counts skipped files and directories', async () => {
    const archive = archiveWith([
      { name: 'ignored/a', data: 'skipped' },
      { name: 'ignored/dir/', type: '5' },
    ]);
    await expect(parseOrgTarball(archive, { ...ORG_TARBALL_LIMITS, entries: 1 })).rejects.toThrow(
      /entry count limit/,
    );
  });

  it('counts metadata records that nanotar does not expose as entries', async () => {
    const archive = archiveWith([
      { name: 'PaxHeader/a', type: 'x', data: paxField('path', 'package/a') },
      { name: 'package/a', data: 'hello' },
    ]);
    await expect(parseOrgTarball(archive, { ...ORG_TARBALL_LIMITS, entries: 1 })).rejects.toThrow(
      /entry count limit/,
    );
  });

  it('limits cumulative extended metadata before nanotar parses it', async () => {
    const archive = archiveWith([
      { name: 'PaxHeader/a', type: 'x', data: paxField('path', 'package/a') },
      { name: 'package/a', data: 'hello' },
      { name: 'PaxHeader/b', type: 'x', data: paxField('path', 'package/b') },
      { name: 'package/b', data: 'hello' },
    ]);
    await expect(
      parseOrgTarball(archive, {
        ...ORG_TARBALL_LIMITS,
        metadataBytes: paxField('path', 'package/a').length,
      }),
    ).rejects.toThrow(/metadata size limit/);
  });

  it('limits cumulative global PAX fields copied into subsequent entries', async () => {
    const archive = archiveWith([
      { name: 'PaxHeader/global-a', type: 'g', data: paxField('comment', 'a') },
      { name: 'PaxHeader/global-b', type: 'g', data: paxField('comment', 'b') },
      { name: 'package/a', data: 'hello' },
    ]);
    await expect(
      parseOrgTarball(archive, { ...ORG_TARBALL_LIMITS, globalMetadataFields: 3 }),
    ).rejects.toThrow(/global metadata field limit/);
  });

  it('applies the compression ratio limit after the allowance for small archives', async () => {
    const archive = archiveWith([{ name: 'package/a', data: 'a'.repeat(16_384) }]);
    await expect(
      parseOrgTarball(archive, {
        ...ORG_TARBALL_LIMITS,
        ratioAllowanceBytes: 1024,
        compressionRatio: 2,
      }),
    ).rejects.toThrow(/decompressed size limit/);
  });

  it('checks UTF-8 path bytes after resolving PAX extended names', async () => {
    const archive = archiveWith([
      { name: 'PaxHeader/a', type: 'x', data: paxField('path', 'package/中文.txt') },
      { name: 'package/a', data: 'hello' },
    ]);
    await expect(
      parseOrgTarball(archive, { ...ORG_TARBALL_LIMITS, pathBytes: 17 }),
    ).rejects.toThrow(/path length limit/);
  });

  it('checks stored extended paths even when normalization shortens them', async () => {
    const archive = archiveWith([
      { name: 'PaxHeader/a', type: 'x', data: paxField('path', `package/${'../'.repeat(30)}a`) },
      { name: 'package/a', data: 'hello' },
    ]);
    await expect(
      parseOrgTarball(archive, { ...ORG_TARBALL_LIMITS, pathBytes: 64 }),
    ).rejects.toThrow(/path length limit/);
  });

  it.each(['x', 'L'])('preserves long paths encoded in %s metadata', async (type) => {
    const name = `package/${'nested/'.repeat(30)}file.txt`;
    const archive = archiveWith([
      { name: 'PaxHeader/a', type, data: type === 'x' ? paxField('path', name) : `${name}\0` },
      { name: 'package/a', data: 'hello' },
    ]);
    const entries = await parseOrgTarball(archive);
    expect(entries).toHaveLength(1);
    expect(entries[0].name).toBe(name);
    expect(entries[0].text).toBe('hello');
  });

  it('rejects a truncated gzip stream', async () => {
    const archive = archiveWith([{ name: 'package/a', data: 'hello' }]);
    await expect(parseOrgTarball(archive.subarray(0, -8))).rejects.toThrow();
  });
});
