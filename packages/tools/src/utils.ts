import { Minimatch } from 'minimatch';

export function replaceUnstableOutput(output: string, cwd?: string) {
  if (cwd) {
    output = output.replaceAll(cwd, '<cwd>');
  }
  return output
    // semver version
    // e.g.: ` v1.0.0` -> ` <semver>`
    // e.g.: `/1.0.0` -> `/<semver>`
    .replaceAll(/([@/\s]v?)\d+\.\d+\.\d+(?:-.*)?/g, '$1<semver>')
    // date
    .replaceAll(/\d{2}:\d{2}:\d{2}/g, '<date>')
    // duration
    .replaceAll(/\d+(?:\.\d+)?(?:s|ms|µs)/g, '<variable>ms')
    // oxlint
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
    .replaceAll(/ ?WARN\s+Skip\s+adding .+?\n/g, '')
    .replaceAll(/ ?WARN\s+Request\s+took .+?\n/g, '')
    .replaceAll(/Scope: all \d+ workspace projects/g, 'Scope: all <variable> workspace projects')
    .replaceAll(/\++\n/g, '+<repeat>\n')
    // ignore yarn YN0013, because it's unstable output, only exists on CI environment
    // ➤ YN0013: │ A package was added to the project (+ 0.7 KiB).
    .replaceAll(/➤ YN0013:[^\n]+\n/g, '')
    // ignore yarn `YN0000: └ Completed <duration>`, it's unstable output
    // ➤ YN0000: └ Completed in <variable>ms <variable>ms
    // ➤ YN0000: └ Completed in <variable>ms
    // =>
    // ➤ YN0000: └ Completed
    .replaceAll(/➤ YN0000: └ Completed.* <variable>(s|ms|µs)( <variable>(s|ms|µs))?\n/g, '➤ YN0000: └ Completed\n')
    // ignore npm warn
    // npm warn Unknown env config "recursive". This will stop working in the next major version of npm
    .replaceAll(/npm warn Unknown env config .+?\n/g, '')
    // WARN  Issue while reading "/path/to/.npmrc". Failed to replace env in config: ${NPM_AUTH_TOKEN}
    .replaceAll(/WARN\s+Issue\s+while\s+reading .+?\n/g, '')
    // ignore npm audited packages log
    // "removed 1 package, and audited 3 packages in 700ms" => "removed <variable> package in <variable>ms"
    // "\nfound 0 vulnerabilities\n" => ""
    .replaceAll(/(removed \d+ package), and audited \d+ packages( in <variable>(?:s|ms|µs))\n/g, '$1$2\n')
    .replaceAll(/\nfound \d+ vulnerabilities\n/g, '')
    // replace size for tsdown
    .replaceAll(/ \d+(\.\d+)? ([km]B)/g, ' <variable> $2');
}

// Exact matches for common environment variables
const DEFAULT_PASSTHROUGH_ENVS = [
  // System and shell
  'HOME',
  'USER',
  'TZ',
  'LANG',
  'SHELL',
  'PWD',
  'PATH',
  // CI/CD environments
  'CI',
  // Node.js specific
  'NODE_OPTIONS',
  'COREPACK_HOME',
  'NPM_CONFIG_STORE_DIR',
  'PNPM_HOME',
  // Library paths
  'LD_LIBRARY_PATH',
  'DYLD_FALLBACK_LIBRARY_PATH',
  'LIBPATH',
  // Terminal/display
  'COLORTERM',
  'TERM',
  'TERM_PROGRAM',
  'DISPLAY',
  'FORCE_COLOR',
  'NO_COLOR',
  // Temporary directories
  'TMP',
  'TEMP',
  // Vercel specific
  'VERCEL',
  'VERCEL_*',
  'NEXT_*',
  'USE_OUTPUT_FOR_EDGE_FUNCTIONS',
  'NOW_BUILDER',
  // Windows specific
  'APPDATA',
  'PROGRAMDATA',
  'SYSTEMROOT',
  'SYSTEMDRIVE',
  'USERPROFILE',
  'HOMEDRIVE',
  'HOMEPATH',
  // IDE specific (exact matches)
  'ELECTRON_RUN_AS_NODE',
  'JB_INTERPRETER',
  '_JETBRAINS_TEST_RUNNER_RUN_SCOPE_TYPE',
  'JB_IDE_*',
  // VSCode specific
  'VSCODE_*',
  // Docker specific
  'DOCKER_*',
  'BUILDKIT_*',
  'COMPOSE_*',
  // Token patterns
  '*_TOKEN',
  // oxc specific
  'OXLINT_*',
].map(env => new Minimatch(env));

export function isPassThroughEnv(env: string) {
  const upperEnv = env.toUpperCase();
  return DEFAULT_PASSTHROUGH_ENVS.some(pattern => pattern.match(upperEnv));
}
