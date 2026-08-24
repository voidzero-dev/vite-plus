const fs = require('fs');

function writeFakeNode(directory, message) {
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(
    `${directory}/fake-node`,
    `#!/usr/bin/env node\nconsole.log(${JSON.stringify(message)});\n`,
    { mode: 0o755 },
  );
  fs.writeFileSync(`${directory}/fake-node.cmd`, '@node "%~dp0\\fake-node" %*\n');
}

writeFakeNode('packages/app/tools', 'resolved from package cwd');
writeFakeNode('packages/shared-tools', 'resolved from parent relative PATH');
