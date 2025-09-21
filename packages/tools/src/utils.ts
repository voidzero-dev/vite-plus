export function replaceUnstableOutput(output: string, cwd?: string) {
  if (cwd) {
    output = output.replaceAll(cwd, '<cwd>');
  }
  return output
    // semver version
    // e.g.: ` v1.0.0` -> ` <semver>`
    // e.g.: `/1.0.0` -> `/<semver>`
    .replaceAll(/([/\s]v?)\d+\.\d+\.\d+(?:-.*)?/g, '$1<semver>')
    // date
    .replaceAll(/\d{2}:\d{2}:\d{2}/g, '<date>')
    // oxlint
    .replaceAll(/\d+(?:\.\d+)?s|\d+ms/g, '<variable>ms')
    .replaceAll(/with \d+ rules/g, 'with <variable> rules')
    .replaceAll(/using \d+ threads/g, 'using <variable> threads')
    // pnpm
    .replaceAll(/Packages: \+\d+/g, 'Packages: +<variable>')
    // only keep done
    .replaceAll(
      /Progress: resolved \d+, reused \d+, downloaded \d+, added \d+, done/g,
      'Progress: resolved <variable>, reused <variable>, downloaded <variable>, added <variable>, done',
    )
    // ignore pnpm progress
    .replaceAll(/Progress: resolved \d+, reused \d+, downloaded \d+, added \d+\n/g, '')
    // ignore pnpm warn
    .replaceAll(/WARN\s+Skip\s+adding .+?\n/g, '')
    .replaceAll(/Scope: all \d+ workspace projects/g, 'Scope: all <variable> workspace projects')
    .replaceAll(/\++\n/g, '+<repeat>\n')
    // replace size for tsdown
    .replaceAll(/ \d+(\.\d+)? ([km]B)/g, ' <variable> $2');
}
