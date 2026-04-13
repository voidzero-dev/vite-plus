// Assert we ARE using the managed Node.js (v22.12.0 from engines.node)
if (process.version !== 'v22.12.0') {
  console.error(`Expected managed Node.js v22.12.0, got ${process.version}`);
  process.exit(1);
}
console.log(`OK: ${process.version}`);
