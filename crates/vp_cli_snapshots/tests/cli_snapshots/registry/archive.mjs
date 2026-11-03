import { gzipSync } from 'node:zlib';

/**
 * Pack small fixture files with fixed ustar metadata and 10 KiB record padding.
 * @param {{ name: string, contents: Buffer, mode: number }[]} files
 * @returns {Buffer}
 */
export function createTarball(files) {
  const blocks = [];
  for (const { name, contents, mode } of files) {
    if (Buffer.byteLength(name) > 100) {
      throw new Error(`Fixture archive path exceeds 100 bytes: ${name}`);
    }
    const header = Buffer.alloc(512);
    header.write(name);
    header.write(mode.toString(8).padStart(7, '0') + '\0', 100);
    header.write('0000000\0', 108);
    header.write('0000000\0', 116);
    header.write(contents.length.toString(8).padStart(11, '0') + '\0', 124);
    header.write('00000000000\0', 136);
    header.fill(' ', 148, 156);
    header.write('0', 156);
    header.write('ustar\0', 257);
    header.write('00', 263);
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
    blocks.push(header, contents, Buffer.alloc((512 - (contents.length % 512)) % 512));
  }
  const archive = Buffer.concat([...blocks, Buffer.alloc(1024)]);
  const padding = Buffer.alloc((10240 - (archive.length % 10240)) % 10240);
  const tarball = gzipSync(Buffer.concat([archive, padding]), { level: 9 });
  // The gzip OS byte must not depend on the machine that packs the fixtures.
  tarball[9] = 255;
  return tarball;
}
