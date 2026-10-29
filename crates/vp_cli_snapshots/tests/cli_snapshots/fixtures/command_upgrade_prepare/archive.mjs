import { gzipSync } from 'node:zlib';

// A local platform archive exercises the real upgrade download and integrity checks.
export function createArchive(files) {
  const blocks = [];
  for (const [name, contents] of Object.entries(files)) {
    const body = Buffer.from(contents);
    const header = Buffer.alloc(512);
    header.write(name);
    header.write('0000755\0', 100);
    header.write(body.length.toString(8).padStart(11, '0') + '\0', 124);
    header.fill(' ', 148, 156);
    header.write('0', 156);
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
    blocks.push(header, body, Buffer.alloc((512 - (body.length % 512)) % 512));
  }
  return gzipSync(Buffer.concat([...blocks, Buffer.alloc(1024)]));
}
