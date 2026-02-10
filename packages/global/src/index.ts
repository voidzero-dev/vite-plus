// Parse command line arguments to intercept 'new', 'migrate', and '--version' commands
// All other commands are delegated to the local CLI
let args = process.argv.slice(2);

// Transform `vp help [command]` into `vp [command] --help`
if (args[0] === 'help' && args[1]) {
  args = [args[1], '--help', ...args.slice(2)];
  process.argv = process.argv.slice(0, 2).concat(args);
}

const command = args[0];

if (command === 'create') {
  await import('./create/bin.js');
} else if (command === 'migrate') {
  await import('./migration/bin.js');
} else if (command === '--version' || command === '-V') {
  await import('./version.js');
} else {
  // Delegate all other commands to local CLI
  await import('./local/bin.js');
}
