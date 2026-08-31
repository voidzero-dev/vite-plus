const { spawnSync } = require('node:child_process')

const component = process.argv[2]
const args = ['env', 'doctor', ...(component ? [component] : [])]
const result = spawnSync('vp', args, { encoding: 'utf8' })
if (result.error)
  throw result.error
if (result.status !== 0) {
  process.stderr.write(result.stdout)
  process.stderr.write(result.stderr)
  process.exit(result.status ?? 1)
}

const output = result.stdout.replace(/\u001b\[[0-9;]*m/g, '')
const lines = output.split('\n')
const start = lines.findIndex(line => line.trim() === 'Configuration')
const endHeading = component ? 'IDE Setup' : 'PATH'
const end = lines.findIndex((line, index) => index > start && line.trim() === endHeading)

console.log(lines.slice(start, end).filter(line => !line.includes('IDE integration')).join('\n').trimEnd())
