import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

const [directory, expected] = process.argv.slice(2)
assert.ok(expected === 'global' || expected === 'local')
const cwd = resolve(directory)
function run(args) {
  const result = spawnSync('vp', args, { cwd, encoding: 'utf8' })
  if (result.error) throw result.error
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`)
  return `${result.stdout}${result.stderr}`
}

const version = run(['--version'])
const delegation = run(['lint', '--help'])
if (expected === 'global') {
  assert.match(version, /Local vite-plus:\s*\n\s*vite-plus\s+Not found/)
  assert.match(delegation, /No project-local vite-plus installation was found/)
  assert.match(delegation, /Usage: vp lint/)
  assert.doesNotMatch(delegation, /Ancestor workspace CLI executed/)
  console.log('Version reports no local CLI; delegation uses the global CLI with an install warning.')
} else {
  assert.match(version, /Local vite-plus:\s*\n\s*vite-plus\s+v9\.8\.7/)
  assert.match(delegation, /Ancestor workspace CLI executed/)
  assert.doesNotMatch(delegation, /No project-local vite-plus installation was found/)
  console.log('Version and delegation use the workspace root CLI without an install warning.')
}
