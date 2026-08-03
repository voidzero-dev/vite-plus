const { promisify } = require('util');
const fs = require('fs');
const path = require('path');

const sleep = promisify(setTimeout);

(async () => {
  if (process.env.VP_TEST_INTERRUPT_INSTALL === '1') {
    fs.writeFileSync(path.join(process.env.VP_HOME, 'env-install-interrupt-ready'), '');
    await sleep(30_000);
  }
})();
