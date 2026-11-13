// Removing the changed line makes both attempts to apply the hidden diff fail.
require('node:fs').writeFileSync('file.txt', 'a\nc\n');
