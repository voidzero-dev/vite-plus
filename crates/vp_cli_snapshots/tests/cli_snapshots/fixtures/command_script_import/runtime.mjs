import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const register = require.resolve('vite-plus/register');
console.log(JSON.stringify({
  node: process.versions.node,
  requireExport: register.replaceAll('\\', '/'),
  importExport: import.meta.resolve('vite-plus/register'),
  sameExport: import.meta.resolve('vite-plus/register') === pathToFileURL(register).href,
}, null, 2));
try {
  console.log('tsx:', require.resolve('tsx'));
} catch (error) {
  console.log('tsx:', error.code);
}
