import { format } from './format.js';

// The declaration generator must infer exported types across module boundaries.
export function greet(name: string) {
  return { message: format(name), length: name.length };
}
