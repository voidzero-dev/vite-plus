// Assert we are NOT using the managed Node.js (system node must win in
// system-first mode). Check execPath instead of the version: the system
// Node.js may have the same version as the engines.node pin.
if (process.execPath.includes('js_runtime')) {
  console.error(`Expected system Node.js, got managed ${process.version}`);
  process.exit(1);
}
console.log(`OK: ${process.version}`);
