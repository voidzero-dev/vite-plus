import { replaceFileContent } from './replace-file-content.js';
import { snapTest } from './snap-test.js';
import { syncRemote } from './sync-remote-deps.js';

const subcommand = process.argv[2];

switch (subcommand) {
  case 'snap-test':
    await snapTest();
    break;
  case 'replace-file-content':
    replaceFileContent();
    break;
  case 'sync-remote':
    syncRemote();
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands: snap-test, replace-file-content, sync-remote');
    process.exit(1);
}
