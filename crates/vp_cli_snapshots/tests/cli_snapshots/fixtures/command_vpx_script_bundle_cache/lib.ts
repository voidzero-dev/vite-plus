import path from 'node:path';

export enum Color {
  Red = 'red',
  Blue = 'blue',
}

export class Store {
  items: number[] = [];

  constructor(private readonly name: string) {}

  add(id: number): void {
    this.items.push(id);
  }

  fail(): never {
    throw new Error(`store ${this.name} failed`);
  }
}

export function where(): string {
  return path.basename(import.meta.dirname);
}
