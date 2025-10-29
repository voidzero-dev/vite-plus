import { replaceFileContent } from './replace-file-content';
import { snapTest } from './snap-test';

const subcommand = process.argv[2];

switch (subcommand) {
  case 'snap-test':
    await snapTest();
    break;
  case 'replace-file-content':
    replaceFileContent();
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands: snap-test, replace-file-content');
    process.exit(1);
}
