// Assert we are NOT using the managed Node.js (v22.18.0 from engines.node)
if (process.version === 'v22.18.0') {
  console.error(`Expected system Node.js, got managed v22.18.0`);
  process.exit(1);
}
console.log(`OK: ${process.version}`);
