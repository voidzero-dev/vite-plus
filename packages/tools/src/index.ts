const subcommand = process.argv[2];

switch (subcommand) {
  case 'sync-remote':
    const { syncRemote } = await import('./sync-remote-deps.ts');
    await syncRemote();
    break;
  case 'install-global-cli':
    const { installGlobalCli } = await import('./install-global-cli.ts');
    installGlobalCli();
    break;
  case 'brand-vite':
    const { brandVite } = await import('./brand-vite.ts');
    brandVite();
    break;
  case 'local-npm-registry':
    // Spawn the script by path instead of importing it, so the child carries
    // the canonical `node .../local-npm-registry.ts` command line that the
    // script's own --ps/--kill maintenance matches.
    const { spawnSync } = await import('node:child_process');
    const { fileURLToPath } = await import('node:url');
    const { exitCodeFromClose } = await import('./exit-code.ts');
    const registryScript = fileURLToPath(new URL('./local-npm-registry.ts', import.meta.url));
    const result = spawnSync(process.execPath, [registryScript, ...process.argv.slice(3)], {
      stdio: 'inherit',
    });
    process.exit(exitCodeFromClose(result.status, result.signal));
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error(
      'Available subcommands: sync-remote, install-global-cli, brand-vite, local-npm-registry',
    );
    process.exit(1);
}

// Can't use top-level await if the file is not a module
