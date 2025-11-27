import { spawn } from 'node:child_process';

async function main() {
  const commands = [
    ['node', ['echo.js', '1'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '2'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '3'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '4'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '5'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '6'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '7'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '8'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '9'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '10'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '11'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '12'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '13'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '14'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '15'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '16'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '17'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '18'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '19'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '20'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '21'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '22'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '23'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '24'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '25'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '26'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '27'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '28'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '29'.repeat(100)], { stdio: 'inherit' }],
    ['node', ['echo.js', '30'.repeat(100)], { stdio: 'inherit' }],
  ];

  console.log('[build.js] --------------------------------');
  console.log('[build.js] start');

  // await Promise.all(commands.map(cmd => exec(...cmd)));
  for (const command of commands) {
    await exec(...command);
  }

  console.log('[build.js] main process end');
}

main().catch(console.error);

/**
 * @param {string} command
 * @param {ReadonlyArray<string>} args
 * @param {object} [options]
 */
export async function exec(command, args, options) {
  return new Promise((resolve, reject) => {
    const _process = spawn(command, args, {
      stdio: [
        'ignore', // stdin
        'pipe', // stdout
        'pipe', // stderr
      ],
      ...options,
      shell: process.platform === 'win32',
    });

    /**
     * @type {Buffer[]}
     */
    const stderrChunks = [];
    /**
     * @type {Buffer[]}
     */
    const stdoutChunks = [];

    _process.stderr?.on('data', (chunk) => {
      stderrChunks.push(chunk);
    });

    _process.stdout?.on('data', (chunk) => {
      stdoutChunks.push(chunk);
    });

    _process.on('error', (error) => {
      reject(error);
    });

    _process.on('exit', (code) => {
      const ok = code === 0;
      const stderr = Buffer.concat(stderrChunks).toString().trim();
      const stdout = Buffer.concat(stdoutChunks).toString().trim();

      if (ok) {
        const result = { ok, code, stderr, stdout };
        resolve(result);
      } else {
        reject(new Error(`Failed to execute command: ${command} ${args.join(' ')}: ${stderr}`));
      }
    });
  });
}
