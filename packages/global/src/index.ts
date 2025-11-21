// Parse command line arguments to intercept 'new', 'gen', and 'migration' commands
const args = process.argv.slice(2);

const command = args[0];
if (command === 'gen' || command === 'g' || command === 'generate' || command === 'new') {
  import('./gen/bin.ts');
} else if (command === 'migration' || command === 'migrate') {
  import('./migration/bin.ts');
} else {
  // Delegate all other commands to vite-plus CLI
  import('@voidzero-dev/vite-plus/bin');
}
