// Use the resolver bundled with the same npm that installs managed globals.
const { createRequire } = require('node:module');
const path = require('node:path');
const npmRequire = createRequire(
  path.resolve(
    path.dirname(process.execPath),
    process.platform === 'win32'
      ? 'node_modules/npm/package.json'
      : '../lib/node_modules/npm/package.json',
  ),
);

async function resolveVersions(rawSpec, command) {
  const Npm = npmRequire('./lib/npm.js');
  // npm 8 reads --global directly from process.argv.
  const npm = new Npm();
  await npm.config.load();

  const parsed = npmRequire('npm-package-arg')(rawSpec);
  const spec = parsed.subSpec ?? parsed;
  const pacote = npmRequire('pacote');
  const options = {
    ...npm.flatOptions,
    fullMetadata: true,
    preferOnline: true,
    _isRoot: true,
    // Both calls below use the same package metadata for a registry spec.
    packumentCache: new Map(),
  };
  // npm versions that support release-age exclusions apply them before fetching the manifest.
  if (options.before && options.minReleaseAgeExclude?.length) {
    const { isReleaseAgeExcluded } = npmRequire('@npmcli/arborist/lib/release-age-exclude.js');
    if (isReleaseAgeExcluded(spec.name, options.minReleaseAgeExclude)) {
      options.before = null;
    }
  }

  const { version: wanted } = await pacote.manifest(spec, options);
  let latest = wanted;
  if (command === 'outdated') {
    const metadata = await pacote.packument(spec.name, options);
    latest = metadata['dist-tags']?.latest;
    if (!latest) {
      throw new Error(`No latest dist-tag for ${spec.name}`);
    }
  }
  process.stdout.write(JSON.stringify({ alias_target: parsed.subSpec?.name, wanted, latest }));
}

const [, rawSpec, command] = process.argv;
resolveVersions(rawSpec, command).catch((error) => {
  process.stderr.write(error.message);
  process.exitCode = 1;
});
