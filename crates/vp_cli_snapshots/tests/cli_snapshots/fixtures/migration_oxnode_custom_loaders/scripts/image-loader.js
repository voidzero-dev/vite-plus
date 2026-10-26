// Representative project loader: turn an image into an ESM value.
import { readFile } from 'node:fs/promises';

export async function load(url, context, nextLoad) {
  if (url.endsWith('.svg')) {
    const text = await readFile(new URL(url), 'utf8');
    return {
      format: 'module',
      shortCircuit: true,
      source: `export default ${JSON.stringify({ name: 'logo.svg', bytes: text.length })};`,
    };
  }
  if (url.endsWith('/generated.cjs')) {
    return {
      format: 'commonjs',
      shortCircuit: true,
      source: `module.exports = { local: require('./dependency.cjs'), builtin: require('node:path').basename('/parent/child') };`,
    };
  }
  const result = await nextLoad(url, context);
  return url.endsWith('/order.js')
    ? { ...result, source: `${result.source}\norder.push('image');` }
    : result;
}
