import { scaffold, type ScaffoldOptions } from './scaffold.ts';

// Parse command line arguments to intercept 'new' command
const args = process.argv.slice(2);

if (args[0] === 'new') {
  // Handle the 'new' command
  const options: ScaffoldOptions = {};
  const positionalArgs: string[] = [];

  // Parse flags and collect positional arguments
  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--help' || args[i] === '-h') {
      options.help = true;
      break;
    } else if (args[i] === '--app' && args[i + 1]) {
      options.app = args[i + 1];
      i++;
    } else if (args[i] === '--lib' && args[i + 1]) {
      options.lib = args[i + 1];
      i++;
    } else if (args[i] === '--monorepo' || args[i] === '-m') {
      options.projectType = 'monorepo';
    } else if (args[i] === '--singlerepo' || args[i] === '-s') {
      options.projectType = 'singlerepo';
    } else if (!args[i].startsWith('-')) {
      positionalArgs.push(args[i]);
    }
  }

  // Handle positional arguments: first is project name, second is project type
  // Skip if help is requested
  if (!options.help) {
    if (positionalArgs.length > 0) {
      options.projectName = positionalArgs[0];
    }
    if (positionalArgs.length > 1) {
      const type = positionalArgs[1].toLowerCase();
      if (type === 'monorepo' || type === 'mono') {
        options.projectType = 'monorepo';
      } else if (type === 'singlerepo' || type === 'single') {
        options.projectType = 'singlerepo';
      }
    }
  }

  // Run scaffolding
  scaffold(options).catch((err) => {
    // Handle SIGINT (Ctrl+C)
    if (err.name === 'AbortError') {
      process.exit(0);
    }
    console.error('[vite+] %s', err);
    process.exit(1);
  });
} else {
  // Delegate all other commands to vite-plus CLI
  import('@voidzero-dev/vite-plus/bin');
}
