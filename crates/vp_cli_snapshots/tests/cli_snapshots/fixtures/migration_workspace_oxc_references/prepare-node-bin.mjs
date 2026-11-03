import fs from 'node:fs';
import path from 'node:path';

// Keep npm exec offline by providing the requested binary in its lookup prefix.
const bin = path.resolve('packages/app/node_modules/.bin');
fs.mkdirSync(bin, { recursive: true });
if (process.platform === 'win32') {
  // npm probes the extensionless file before cmd.exe selects the .cmd launcher.
  fs.writeFileSync(
    path.join(bin, 'node'),
    `#!/bin/sh\nexec "${process.execPath.replaceAll('\\', '/')}" "$@"\n`,
  );
  fs.writeFileSync(path.join(bin, 'node.cmd'), `@"${process.execPath}" %*\r\n`);
} else {
  fs.symlinkSync(process.execPath, path.join(bin, 'node'));
}
