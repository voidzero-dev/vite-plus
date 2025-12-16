// Parse command line arguments to intercept 'new', 'gen', and 'migration' commands
const args = process.argv.slice(2);

const LOCAL_CLI_COMMANDS = ['dev', 'build', 'test', 'lint', 'fmt', 'format', 'lib', 'doc', 'run'];

const command = args[0];

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
