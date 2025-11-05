// Parse command line arguments to intercept 'new' and 'gen' commands
const args = process.argv.slice(2);

const command = args[0];
if (command === 'gen' || command === 'g' || command === 'generate' || command === 'new') {
  import('./gen.ts');
} else {
  // Delegate all other commands to vite-plus CLI
  import('@voidzero-dev/vite-plus/bin');
}
