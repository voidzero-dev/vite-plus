// CommonJS that require()s another CommonJS file, imported from ESM.
const legacy = require('./legacy.cts');

module.exports = { name: `nested ${legacy.name}` };
