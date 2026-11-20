const { parentPort } = require('node:worker_threads');
const common = require('./common.cts');
enum Result { Value = 'cjs worker' }
parentPort.postMessage(`${Result.Value}: ${common.format}`);
