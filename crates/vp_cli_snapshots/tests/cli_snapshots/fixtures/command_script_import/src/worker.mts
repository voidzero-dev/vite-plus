import { parentPort } from 'node:worker_threads';
import common from './common.cts';
import { value } from '@value';
enum Result { Value = 'esm worker' }
parentPort!.postMessage(`${Result.Value}: ${common.format}: ${value}`);
