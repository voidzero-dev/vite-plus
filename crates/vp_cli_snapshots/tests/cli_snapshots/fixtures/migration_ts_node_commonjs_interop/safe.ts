import value, { value as named } from './esm/dep.js';
import * as dependency from './esm/dep.js';
export { value } from './esm/dep.js';
export * as namespace from './esm/dep.js';
export * from './esm/dep.js';
console.log(value, named, dependency.value);
