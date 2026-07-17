const subcommand = process.argv[2];

switch (subcommand) {
  case 'replace-file-content':
    const { replaceFileContent } = await import('./replace-file-content.ts');
    replaceFileContent();
    break;
  case 'sync-remote':
    const { syncRemote } = await import('./sync-remote-deps.ts');
    await syncRemote();
    break;
  case 'json-sort':
    const { jsonSort } = await import('./json-sort.ts');
    jsonSort();
    break;
  case 'merge-peer-deps':
    const { mergePeerDeps } = await import('./merge-peer-deps.ts');
    mergePeerDeps();
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
    const registryScript = fileURLToPath(new URL('./local-npm-registry.ts', import.meta.url));
    const result = spawnSync(process.execPath, [registryScript, ...process.argv.slice(3)], {
      stdio: 'inherit',
    });
    process.exit(result.status ?? 1);
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error(
      'Available subcommands: replace-file-content, sync-remote, json-sort, merge-peer-deps, install-global-cli, brand-vite, local-npm-registry',
    );
    process.exit(1);
}

// Can't use top-level await if the file is not a module
