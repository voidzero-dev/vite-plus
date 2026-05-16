import { readFile, writeFile } from 'node:fs/promises';

import { createTarGzip, parseTarGzip, type TarFileInput } from 'nanotar';

export async function repackViteTgz() {
  const [inputPath, outputPath, newName] = process.argv.slice(3);

  if (!inputPath || !outputPath || !newName) {
    console.error('Usage: tool repack-vite-tgz <input.tgz> <output.tgz> <new-name>');
    process.exit(1);
  }

  const inputBytes = await readFile(inputPath);
  const entries = await parseTarGzip(inputBytes);

  let patched = 0;
  const repacked: TarFileInput[] = entries.map((entry) => {
    let data = entry.data;
    if (entry.name === 'package/package.json' && data) {
      const pkg = JSON.parse(new TextDecoder().decode(data)) as Record<string, unknown>;
      pkg.name = newName;
      data = new TextEncoder().encode(JSON.stringify(pkg, null, 2) + '\n');
      patched += 1;
    }
    return { name: entry.name, data, attrs: entry.attrs };
  });

  if (patched !== 1) {
    console.error(`Expected exactly one package/package.json entry, found ${patched}`);
    process.exit(1);
  }

  const outBytes = await createTarGzip(repacked);
  await writeFile(outputPath, outBytes);
  console.log(
    `Repacked ${inputPath} -> ${outputPath} (name=${newName}, ${outBytes.byteLength} bytes)`,
  );
}
