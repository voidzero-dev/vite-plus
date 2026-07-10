// Assert we ARE using the managed Node.js (v22.18.0 from engines.node).
// Check execPath in addition to the version: the system Node.js may have
// the same version as the pinned one (e.g. CI installs .node-version).
if (!process.execPath.includes('js_runtime') || process.version !== 'v22.18.0') {
  console.error(`Expected managed Node.js v22.18.0, got ${process.version}`);
  process.exit(1);
}
console.log(`OK: ${process.version}`);
