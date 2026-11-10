// A published TypeScript dependency loaded with require(), through the CommonJS hook.
const { kind } = require('ts-cjs-dep');

console.log('ts-cjs-dep:', kind);
