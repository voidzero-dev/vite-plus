// Node.js 26 runs the hooks in-thread through module.registerHooks().
import { createRequire } from 'node:module';

enum Path {
  Sync = 'registerHooks',
}

class Resource {
  disposed = false;
  [Symbol.dispose]() {
    this.disposed = true;
  }
}

const resource = new Resource();
{
  using held = resource;
  void held;
}

const require = createRequire(import.meta.url);
console.log(Path.Sync, require('./legacy.cts').name, resource.disposed);
