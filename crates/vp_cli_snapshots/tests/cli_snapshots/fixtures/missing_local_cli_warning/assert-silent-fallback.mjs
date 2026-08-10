import { spawnSync } from 'node:child_process'

const result = spawnSync('vp', ['lint', '--help'], { encoding: 'utf8' })
const output = `${result.stdout}${result.stderr}`

if (result.error)
  throw result.error

if (result.status !== 0 || !output.includes('Usage: vp lint'))
  throw new Error(`Global CLI did not run successfully:\n${output}`)

if (output.includes('No project-local vite-plus installation was found'))
  throw new Error(`Unexpected missing local CLI warning:\n${output}`)

console.log('Global fallback remained silent.')
