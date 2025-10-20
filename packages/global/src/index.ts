// Parse command line arguments to intercept 'new' command
const args = process.argv.slice(2);

if (args[0] === 'new') {
  import('./new.ts');
} else {
  // Delegate all other commands to vite-plus CLI
  import('@voidzero-dev/vite-plus/bin');
}
