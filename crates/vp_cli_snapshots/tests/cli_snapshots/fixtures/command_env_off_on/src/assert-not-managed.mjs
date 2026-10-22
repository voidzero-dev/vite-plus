// Assert we are using the system Node.js from `system-bin` (system node must
// win in system-first mode). Check execPath instead of the version: the
// system Node.js may have the same version as the engines.node pin.
if (!process.execPath.includes('system-bin')) {
  console.error(`Expected system Node.js, got ${process.version} from a non-system path`);
  process.exit(1);
}
console.log(`OK: system ${process.version}`);
