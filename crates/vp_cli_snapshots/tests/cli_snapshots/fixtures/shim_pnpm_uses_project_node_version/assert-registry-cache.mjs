import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const manager = process.argv[2];
const registry = process.env.npm_config_registry ?? process.env.NPM_CONFIG_REGISTRY;
const count = async () => (await (await fetch(`${registry}/-/vp-test/requests`)).json()).requests;
const before = await count();
const actual = execFileSync(manager, ['--version'], { encoding: 'utf8' });
const after = await count();
if (before !== after) throw new Error(`${manager} contacted the registry despite its fresh cache`);
if (manager === 'bun') {
  const expected = readFileSync(`${process.env.VP_HOME}/bun-fallback-version`, 'utf8');
  if (actual !== expected) throw new Error(`expected ${expected.trim()}, got ${actual.trim()}`);
  console.log('default bun@latest matches the unconfigured fallback');
}
