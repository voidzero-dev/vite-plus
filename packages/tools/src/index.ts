const subcommand = process.argv[2];

switch (subcommand) {
  case 'snap-test':
    const { snapTest } = await import('./snap-test');
    await snapTest();
    break;
  case 'replace-file-content':
    const { replaceFileContent } = await import('./replace-file-content');
    replaceFileContent();
    break;
  case 'sync-remote':
    const { syncRemote } = await import('./sync-remote-deps');
    syncRemote();
    break;
  case 'json-sort':
    const { jsonSort } = await import('./json-sort');
    jsonSort();
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands: snap-test, replace-file-content, sync-remote, json-sort');
    process.exit(1);
}
