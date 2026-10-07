class Counter {
  count = 1;
}

export function describe(): string {
  return `cts with export, counter=${new Counter().count}`;
}
