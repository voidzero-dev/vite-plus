const { promisify } = require('util');
const fs = require('fs');
const path = require('path');

const sleep = promisify(setTimeout);

(async () => {
  if (process.env.VP_TEST_INTERRUPT_INSTALL === '1') {
    // The PID lets the interrupt driver kill this subtree directly instead
    // of enumerating vp's children through PowerShell/CIM, whose cold start
    // on a loaded runner can outlast the step timeout. Write-then-rename so
    // the driver never observes the file without the PID in it.
    const readyPath = path.join(process.env.VP_HOME, 'env-install-interrupt-ready');
    fs.writeFileSync(`${readyPath}.tmp`, String(process.pid));
    fs.renameSync(`${readyPath}.tmp`, readyPath);
    await sleep(30_000);
  }
})();
