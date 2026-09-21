const { existsSync, readdirSync } = require('node:fs');
const path = require('node:path');

// Normalize executable suffixes and omit Windows sidecars, keeping placement visible in snapshots.
for (const directory of process.argv.slice(2)) {
  const resolved = directory.replace('$VP_HOME', process.env.VP_HOME);
  const names = readdirSync(resolved).filter((name) =>
    /^(node|npm|npx|pnpm|pnpx|pn|pnx|yarn|yarnpkg|bun|bunx|vpx|vpr)(\.exe)?$/.test(name),
  );
  for (const name of names.sort()) {
    if (!existsSync(path.join(resolved, name))) throw new Error(`Broken shim: ${name}`);
  }
  console.log(`${directory}: ${names.map((name) => name.replace(/\.exe$/, '')).join(', ') || '(empty)'}`);
}
