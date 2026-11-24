// An ES module dependency that uses a free `require()`, which Node.js does not define for
// it: the `catch` path runs.
import { path } from 'uses-require';

console.log(`path=${path}`);
