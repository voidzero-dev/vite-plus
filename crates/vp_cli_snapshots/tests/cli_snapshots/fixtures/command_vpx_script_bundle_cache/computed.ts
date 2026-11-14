import { counter } from './shared.ts';

export function computed(): string {
  counter.value += 10;
  return 'loaded';
}
