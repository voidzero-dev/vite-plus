import { homedir } from 'node:os';
import path from 'node:path';

import { Minimatch } from 'minimatch';

export function replaceUnstableOutput(output: string, cwd?: string) {
  if (cwd) {
    output = output.replaceAll(cwd, '<cwd>');
    if (path.dirname(cwd) !== '/') {
      output = output.replaceAll(path.dirname(cwd), '<cwd>/..');
    }
  }
  // On Windows, normalize path separators in file paths for consistent snapshots.
  // Only replace backslashes that look like path separators (preceded/followed by valid path chars).
  // This avoids breaking ASCII art or escape sequences.
  if (process.platform === 'win32') {
    // Replace backslashes in patterns like: word\word, ./path\to, src\file.ts
    // Pattern: backslash between alphanumeric/dot/underscore/hyphen chars
    output = output.replaceAll(/([a-zA-Z0-9._-])\\([a-zA-Z0-9._-])/g, '$1/$2');
  }
  return (
    output
      // semver version
      // e.g.: ` v1.0.0` -> ` <semver>`
      // e.g.: `/1.0.0` -> `/<semver>`
      .replaceAll(/([@/\s]v?)\d+\.\d+\.\d+(?:-.*)?/g, '$1<semver>')
      // vite-plus hash version
      // e.g.: `vite-plus": "^0.0.0-aa9f90fe23216b8ad85b0ba4fc1bccb0614afaf0"` -> `vite-plus": "^0.0.0-<hash>`
      .replaceAll(/0\.0\.0-\w{40}/g, '0.0.0-<hash>')
      // date
      .replaceAll(/\d{2}:\d{2}:\d{2}/g, '<date>')
      // duration
      .replaceAll(/\d+(?:\.\d+)?(?:s|ms|µs|ns)/g, '<variable>ms')
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
      // ignore pnpm registry request error warning log
      .replaceAll(/ ?WARN\s+GET\s+https:\/\/registry\..+?\n/g, '')
      // ignore yarn YN0013, because it's unstable output, only exists on CI environment
      // ➤ YN0013: │ A package was added to the project (+ 0.7 KiB).
      .replaceAll(/➤ YN0013:[^\n]+\n/g, '')
      // ignore yarn `YN0000: └ Completed <duration>`, it's unstable output
      // ➤ YN0000: └ Completed in <variable>ms <variable>ms
      // ➤ YN0000: └ Completed in <variable>ms
      // =>
      // ➤ YN0000: └ Completed
      .replaceAll(
        /➤ YN0000: └ Completed.* <variable>(s|ms|µs)( <variable>(s|ms|µs))?\n/g,
        '➤ YN0000: └ Completed\n',
      )
      // ignore npm warn
      // npm warn Unknown env config "recursive". This will stop working in the next major version of npm
      .replaceAll(/npm warn Unknown env config .+?\n/g, '')
      // WARN  Issue while reading "/path/to/.npmrc". Failed to replace env in config: ${NPM_AUTH_TOKEN}
      .replaceAll(/WARN\s+Issue\s+while\s+reading .+?\n/g, '')
      // ignore npm audited packages log
      // "removed 1 package, and audited 3 packages in 700ms" => "removed <variable> package in <variable>ms"
      // "up to date, audited 4 packages in 11ms" => "up to date in <variable>ms"
      // "added 3 packages, and audited 4 packages in 100ms" => "added 3 packages in <variable>ms"
      // "\nfound 0 vulnerabilities\n" => ""
      .replaceAll(
        /(removed \d+ package), and audited \d+ packages( in <variable>(?:s|ms|µs))\n/g,
        '$1$2\n',
      )
      .replaceAll(/(up to date), audited \d+ packages( in <variable>(?:s|ms|µs))\n/g, '$1$2\n')
      .replaceAll(
        /(added \d+ packages?), and audited \d+ packages( in <variable>(?:s|ms|µs))\n/g,
        '$1$2\n',
      )
      .replaceAll(/\nfound \d+ vulnerabilities\n/g, '')
      // replace size for tsdown
      .replaceAll(/ \d+(\.\d+)? ([km]?B)/g, ' <variable> $2')
      // replace npm notice size:
      // "npm notice 5.6kB snap.txt"
      // "npm notice 619B steps.json"
      .replaceAll(/ \d+(\.\d+)?([km]?B) /g, ' <variable>$2 ')
      // '"size": 821' => '"size": <variable>'
      // '"unpackedSize": 2720' => '"unpackedSize": <variable>'
      .replaceAll(/"(size|unpackedSize)": \d+/g, '"$1": <variable>')
      // ignore npm registry domain
      .replaceAll(/(https?:\/\/registry\.)[^/\s]+(\/?)/g, '$1<domain>$2')
      // ignore pnpm tarball download average speed warning log
      .replaceAll(/ WARN  Tarball download average speed .+?\n/g, '')
      // ignore npm hash values
      .replaceAll(/shasum: .+?\n/g, 'shasum: <hash>\n')
      .replaceAll(/integrity: ([\w-]+)-.+?\n/g, 'integrity: $1-<hash>\n')
      .replaceAll(/"shasum": ".+?"/g, '"shasum": "<hash>"')
      .replaceAll(/"integrity": "(\w+)-.+?"/g, '"integrity": "$1-<hash>"')
      // replace homedir; e.g.: /Users/foo/Library/pnpm/global/5/node_modules/testnpm2 => <homedir>/Library/pnpm/global/5/node_modules/testnpm2
      .replaceAll(homedir(), '<homedir>')
      // remove the newline after "Checking formatting..."
      .replaceAll(`Checking formatting...\n`, 'Checking formatting...')
      // remove warning <name>@<semver>: No license field
      .replaceAll(/warning .+?: No license field\n/g, '')
      // remove "npm warn exec The following package was not found and will be installed: cowsay@<semver>"
      .replaceAll(
        /npm warn exec The following package was not found and will be installed: .+?\n/g,
        '',
      )
      // remove "npm notice Access token expired or revoked..."
      .replaceAll(/npm notice Access token expired or revoked.+?\n/g, '')
      // remove mise reshimming messages (appears when global npm packages change)
      .replaceAll(/Reshimming mise.+?\n/g, '')
      // remove plugin timings warnings (intermittent CI warnings)
      // [PLUGIN_TIMINGS] Warning: Your build spent significant time in plugins. Here is a breakdown:
      //   - externalize-deps (74%)
      .replaceAll(/\[PLUGIN_TIMINGS\] Warning:.*?\n(?:\s+-\s+.+?\n)*/g, '')
  );
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
  'PATHEXT', // .EXE;.BAT;...
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
  // Rust specific
  'RUST_*',
  // Vite specific
  'VITE_*',
].map((env) => new Minimatch(env));

export function isPassThroughEnv(env: string) {
  const upperEnv = env.toUpperCase();
  return DEFAULT_PASSTHROUGH_ENVS.some((pattern) => pattern.match(upperEnv));
}
