import t from '@bomb.sh/tab';

// Parse command line arguments to intercept 'new', 'gen', and 'migration' commands
const args = process.argv.slice(2);

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
];

const command = args[0];

// Define your CLI structure
const devCmd = t.command('dev', 'Start development server');
devCmd.option('port', 'Specify port', (complete) => {
  complete('5173', 'Default port');
  complete('8080', 'Rich port');
});

// Handle completion requests
if (command === 'complete') {
  const shell = process.argv[3];
  if (shell === '--') {
    const args = process.argv.slice(4);
    t.parse(args);
  } else {
    t.setup('vite', 'vite', shell);
  }
  process.exit(0);
}

if (command === 'gen' || command === 'g' || command === 'generate' || command === 'new') {
  import('./gen/bin.js');
} else if (command === 'migration' || command === 'migrate') {
  import('./migration/bin.js');
} else if (LOCAL_CLI_COMMANDS.includes(command)) {
  import('./local/bin.js');
} else if (command === '--version' || command === '-V') {
  import('./version.js');
} else {
  // Delegate to rust commands
  import('./global/bin.js');
}
