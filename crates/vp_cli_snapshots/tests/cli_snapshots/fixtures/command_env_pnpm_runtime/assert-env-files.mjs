import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const mode = process.argv[2]
const expected = {
  managed: {
    env: 'export PNPM_CONFIG_RUNTIME=false',
    'env.fish': 'set -gx PNPM_CONFIG_RUNTIME false',
    'env.nu': '$env.PNPM_CONFIG_RUNTIME = "false"',
    'env.ps1': '$env:PNPM_CONFIG_RUNTIME = "false"',
  },
  'system-first': {
    env: 'unset PNPM_CONFIG_RUNTIME',
    'env.fish': 'set -e PNPM_CONFIG_RUNTIME',
    'env.nu': 'if ("PNPM_CONFIG_RUNTIME" in $env) { hide-env PNPM_CONFIG_RUNTIME }',
    'env.ps1': 'Remove-Item Env:\\PNPM_CONFIG_RUNTIME -ErrorAction SilentlyContinue',
  },
}[mode]

if (!expected)
  throw new Error(`Unknown mode: ${mode}`)

for (const [file, line] of Object.entries(expected)) {
  const setup = readFileSync(join(process.env.VP_HOME, file), 'utf8').split('\n').slice(0, 6)
  if (!setup.includes(line))
    throw new Error(`${file} does not configure pnpm for ${mode} mode`)
}

console.log(`All shell environments match ${mode} mode`)
