import { jsonEdit } from './json-edit';
import { replaceFileContent } from './replace-file-content';
import { snapTest } from './snap-test';

const subcommand = process.argv[2];

switch (subcommand) {
  case 'json-edit':
    jsonEdit();
    break;
  case 'snap-test':
    await snapTest();
    break;
  case 'replace-file-content':
    replaceFileContent();
    break;
  default:
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands: json-edit, snap-test, replace-file-content, sync-remote');
    process.exit(1);
}
