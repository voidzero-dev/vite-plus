// Runs lint-staged on staged files using the programmatic API.
// Bundled by rolldown — no runtime dependency needed in user projects.
//
// We use the programmatic API instead of importing lint-staged/bin because
// lint-staged's dependency tree includes CJS modules that use require('node:events')
// etc., which breaks when bundled to ESM format by rolldown.
import lintStaged from 'lint-staged';
import mri from 'mri';

import { renderCliDoc } from '../utils/help.js';
import { getVitePlusHeader, log } from '../utils/terminal.js';

const helpMessage = renderCliDoc({
  usage: 'vp lint-staged',
  summary: 'Run linters on staged files.',
  sections: [
    {
      title: 'Options',
      rows: [{ label: '-h, --help', description: 'Show this help message' }],
    },
  ],
});

const args = mri(process.argv.slice(3), {
  alias: { h: 'help' },
  boolean: ['help'],
});

if (args.help) {
  log((await getVitePlusHeader()) + '\n');
  log(helpMessage);
} else {
  const success = await lintStaged({});
  process.exit(success ? 0 : 1);
}
