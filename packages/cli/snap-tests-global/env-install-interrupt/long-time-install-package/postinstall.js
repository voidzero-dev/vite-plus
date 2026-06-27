const { promisify } = require('util');

const sleep = promisify(setTimeout);

(async () => {
  await sleep(200);
})();
