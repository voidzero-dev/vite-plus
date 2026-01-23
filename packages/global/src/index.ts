// Parse command line arguments to intercept 'new', and 'migrate' commands
let args = process.argv.slice(2);

if (args[0] === 'help' && args[1]) {
  args = [args[1], '--help', ...args.slice(2)];
  process.argv = process.argv.slice(0, 2).concat(args);
}

const LOCAL_CLI_COMMANDS = [
  'dev',
  'build',
  'test',
  'lint',
  'fmt',
  'format',
  'lib',
  'doc',
  'run',
  'preview',
  'cache',
];

const command = args[0];

if (command === 'new') {
  import('./new/bin.js');
} else if (command === 'migrate') {
  import('./migration/bin.js');
} else if (LOCAL_CLI_COMMANDS.includes(command)) {
  import('./local/bin.js');
} else if (command === '--version' || command === '-V') {
  import('./version.js');
} else {
  // Delegate to rust commands
  import('./global/bin.js');
}
