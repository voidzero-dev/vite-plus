const { spawnSync } = require('node:child_process')

const result = spawnSync('vp', ['env', 'doctor', 'pnpm'], { encoding: 'utf8' })
const output = result.stdout.replace(/\u001B\[[0-9;]*m/g, '')
if (!output.includes('Source') || !output.includes('system PATH') || !output.includes('PM binary') || !output.includes('system-bin/pnpm')) {
  process.stderr.write(`${result.stdout}${result.stderr}`)
  process.exit(1)
}

console.log('doctor reports the system pnpm binary without resolving the declared range')

const current = spawnSync('vp', ['env', 'current', 'pnpm', '--json'], { encoding: 'utf8' })
if (current.status !== 0) {
  process.stderr.write(`${current.stdout}${current.stderr}`)
  process.exit(1)
}
const info = JSON.parse(current.stdout)
if (info.package_manager?.source !== 'system PATH' || !info.package_manager?.bin_paths?.pnpm?.includes('system-bin/pnpm')) {
  process.stderr.write(current.stdout)
  process.exit(1)
}

console.log('current reports the system pnpm binary without resolving the declared range')
