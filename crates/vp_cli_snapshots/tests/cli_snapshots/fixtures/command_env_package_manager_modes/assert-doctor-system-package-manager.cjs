const { spawnSync } = require('node:child_process')

const result = spawnSync('vp', ['env', 'doctor', 'pnpm'], { encoding: 'utf8' })
const output = result.stdout.replace(/\u001B\[[0-9;]*m/g, '')
if (!output.includes('Source') || !output.includes('system PATH') || !output.includes('PM binary') || !output.includes('system-bin/pnpm')) {
  process.stderr.write(`${result.stdout}${result.stderr}`)
  process.exit(1)
}

console.log('doctor reports the system pnpm binary without resolving the declared range')
