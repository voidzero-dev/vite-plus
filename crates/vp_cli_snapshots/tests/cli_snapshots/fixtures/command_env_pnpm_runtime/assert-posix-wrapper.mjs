import { spawnSync } from 'node:child_process'

const script = [
  '. "$VP_HOME/env"',
  '[ "$PNPM_CONFIG_RUNTIME" = false ] || exit 1',
  'vp env off >/dev/null || exit 1',
  '[ -z "${PNPM_CONFIG_RUNTIME+x}" ] || exit 1',
  'vp env on >/dev/null || exit 1',
  '[ "$PNPM_CONFIG_RUNTIME" = false ] || exit 1',
].join('\n')
const result = spawnSync('/bin/sh', ['-c', script], { env: process.env, encoding: 'utf8' })

if (result.status !== 0)
  throw new Error(result.stderr || `shell exited with ${result.status}`)

console.log('Current shell follows Vite+ environment mode')
