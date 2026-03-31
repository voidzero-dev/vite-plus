const subcommand = process.argv[2];

switch (subcommand) {
  case 'snap-test':
    const { snapTest } = await import('./snap-test.js');
    await snapTest();
    break;
  case 'replace-file-content':
    const { replaceFileContent } = await import('./replace-file-content.js');
    replaceFileContent();
    break;
  case 'sync-remote':
    const { syncRemote } = await import('./sync-remote-deps.js');
    await syncRemote();
    break;
  case 'json-sort':
    const { jsonSort } = await import('./json-sort.js');
    jsonSort();
    break;
  case 'merge-peer-deps':
    const { mergePeerDeps } = await import('./merge-peer-deps.js');
    mergePeerDeps();
    break;
  case 'install-global-cli':
    const { installGlobalCli } = await import('./install-global-cli.js');
    installGlobalCli();
    break;
  case 'brand-vite':
    const { brandVite } = await import('./brand-vite.js');
    brandVite();
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error(
      'Available subcommands: snap-test, replace-file-content, sync-remote, json-sort, merge-peer-deps, install-global-cli, brand-vite',
    );
    process.exit(1);
}

// Can't use top-level await if the file is not a module
