const { spawnSync } = require('node:child_process')

const result = spawnSync('vp', ['env', 'doctor'], { encoding: 'utf8' })
if (result.error)
  throw result.error

const output = result.stdout.replace(/\u001b\[[0-9;]*m/g, '')
const lines = output.split('\n')
const start = lines.findIndex(line => line.trim() === 'Configuration')
const end = lines.findIndex((line, index) => index > start && line.trim() === 'PATH')

console.log(lines.slice(start, end).filter(line => !line.includes('IDE integration')).join('\n').trimEnd())
