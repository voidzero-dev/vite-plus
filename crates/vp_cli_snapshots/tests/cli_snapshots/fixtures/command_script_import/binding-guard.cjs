// An unsupported runtime must stop before loading a native binding.
const Module = require('node:module');
const original = Module._load;
Module._load = function (request, ...args) {
  if (/binding[\\/]index\.(?:cjs|js)$/.test(request) || request.endsWith('.node')) {
    throw new Error('NATIVE_BINDING_LOADED_BEFORE_VERSION_CHECK');
  }
  return original.call(this, request, ...args);
};
