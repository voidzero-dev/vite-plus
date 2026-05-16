import { readFile, writeFile } from 'node:fs/promises';

import { createTarGzip, parseTarGzip, type TarFileInput } from 'nanotar';

interface PackageJson {
  name?: string;
  version?: string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
}

function stripVitePlusCoreSelfRefs(pkg: PackageJson, newName: string): void {
  for (const field of [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'optionalDependencies',
  ] as const) {
    const group = pkg[field];
    if (!group) {
      continue;
    }
    for (const [key, value] of Object.entries(group)) {
      if (
        (key === newName || key === '@voidzero-dev/vite-plus-core') &&
        typeof value === 'string' &&
        value.includes('@voidzero-dev/vite-plus-core')
      ) {
        delete group[key];
      }
    }
  }
}

export async function repackViteTgz() {
  const [inputPath, outputPath, newName, newVersion] = process.argv.slice(3);

  if (!inputPath || !outputPath || !newName) {
    console.error('Usage: tool repack-vite-tgz <input.tgz> <output.tgz> <new-name> [new-version]');
    process.exit(1);
  }

  const inputBytes = await readFile(inputPath);
  const entries = await parseTarGzip(inputBytes);

  let patched = 0;
  const repacked: TarFileInput[] = entries.map((entry) => {
    let data = entry.data;
    if (entry.name === 'package/package.json' && data) {
      const pkg = JSON.parse(new TextDecoder().decode(data)) as PackageJson;
      pkg.name = newName;
      if (newVersion) {
        pkg.version = newVersion;
      }
      // Strip any self-ref (`vite: npm:@voidzero-dev/vite-plus-core@...`)
      // injected by the workspace-level vite -> vite-plus-core override at
      // pnpm pack time. Once we rename the tgz to "vite", this becomes a
      // circular self-dependency that confuses pnpm's resolver and triggers a
      // registry lookup for the alias target version.
      stripVitePlusCoreSelfRefs(pkg, newName);
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
    `Repacked ${inputPath} -> ${outputPath} (name=${newName}${
      newVersion ? `, version=${newVersion}` : ''
    }, ${outBytes.byteLength} bytes)`,
  );
}
