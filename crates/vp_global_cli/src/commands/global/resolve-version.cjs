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

async function resolveVersion(rawSpec) {
  const Npm = npmRequire('./lib/npm.js');
  const npm = new Npm({ argv: ['--global'] });
  await npm.config.load();

  const spec = npmRequire('npm-package-arg')(rawSpec);
  if (!spec.registry) {
    throw new Error('Expected a registry package');
  }

  const options = npm.flatOptions;
  // npm versions that support release-age exclusions apply them before fetching the manifest.
  if (options.before && options.minReleaseAgeExclude?.length) {
    const { isReleaseAgeExcluded } = npmRequire('@npmcli/arborist/lib/release-age-exclude.js');
    if (isReleaseAgeExcluded(spec.name, options.minReleaseAgeExclude)) {
      options.before = null;
    }
  }

  const manifest = await npmRequire('pacote').manifest(spec, {
    ...options,
    fullMetadata: true,
    preferOnline: true,
    _isRoot: true,
  });
  process.stdout.write(manifest.version);
}

resolveVersion(process.argv[1]).catch((error) => {
  process.stderr.write(error.message);
  process.exitCode = 1;
});
