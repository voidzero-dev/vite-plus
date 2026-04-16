const fs = require('node:fs');
const path = require('node:path');

// Reproduce issue #1374: build fails when project root is reached through an NTFS junction.
// Layout after setup:
//   ./real/app           <- the actual project (real path)
//   ./via                <- NTFS junction pointing to ./real
//   ./via/app            <- the project reached through the junction
fs.symlinkSync(path.resolve('real'), path.resolve('via'), 'junction');
