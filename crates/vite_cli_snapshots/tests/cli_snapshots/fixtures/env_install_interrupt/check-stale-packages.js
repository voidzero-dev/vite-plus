const fs = require('fs');
const path = require('path');

const packageBase = 'long-time-install-package';
const scopeDir = path.join(process.env.VP_HOME, 'packages', '@scope');
const metadataPath = path.join(scopeDir, `${packageBase}.json`);
const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
const activeDir = `${packageBase}${metadata.installId}`;
const expectStale = process.argv.includes('--expect-stale');

const packageDirs = fs
  .readdirSync(scopeDir, { withFileTypes: true })
  .filter((entry) => {
    if (!entry.isDirectory()) {
      return false;
    }
    if (entry.name === packageBase) {
      return true;
    }
    return /^long-time-install-package#[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
      entry.name,
    );
  })
  .map((entry) => entry.name)
  .sort();

const hasIdentifiedStale = packageDirs.some((name) => name !== packageBase && name !== activeDir);

console.log(
  hasIdentifiedStale ? 'interrupted stale package exists' : 'interrupted stale package removed',
);

if (expectStale !== hasIdentifiedStale) {
  process.exit(1);
}
