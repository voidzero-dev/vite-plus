// Parse command line arguments to intercept 'new', 'migrate', and '--version' commands
// All other commands are delegated to the local CLI
const command = process.argv[2];

if (command === 'new') {
  import('./new/bin.js');
} else if (command === 'migrate') {
  import('./migration/bin.js');
} else if (command === '--version' || command === '-V') {
  import('./version.js');
} else {
  // Delegate all other commands to local CLI
  import('./local/bin.js');
}
