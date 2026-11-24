import { counter } from './shared.ts';

export function lazy(): string {
  counter.value++;
  return 'loaded';
}
