// Parse command line arguments to intercept 'new' command
const args = process.argv.slice(2);

if (args[0] === 'new') {
  import('./new.ts');
} else {
  // Delegate all other commands to vite-plus CLI
  // @ts-ignore no types for vite-plus/bin
  import('@voidzero-dev/vite-plus/bin');
}
