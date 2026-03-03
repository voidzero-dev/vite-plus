// Runs lint-staged on staged files using the programmatic API.
// Bundled by rolldown — no runtime dependency needed in user projects.
//
// We use the programmatic API instead of importing lint-staged/bin because
// lint-staged's dependency tree includes CJS modules that use require('node:events')
// etc., which breaks when bundled to ESM format by rolldown.
import lintStaged from 'lint-staged';

const success = await lintStaged({});
process.exit(success ? 0 : 1);
