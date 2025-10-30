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
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands: snap-test, replace-file-content, sync-remote');
    process.exit(1);
}
