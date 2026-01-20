const subcommand = process.argv[2];

switch (subcommand) {
  case 'snap-test':
    const { snapTest } = await import('./snap-test.ts');
    await snapTest();
    break;
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
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error(
      'Available subcommands: snap-test, replace-file-content, sync-remote, json-sort, merge-peer-deps, install-global-cli',
    );
    process.exit(1);
}

// Can't use top-level await if the file is not a module
export {};
