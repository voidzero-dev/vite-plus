class Counter {
  count = 1;
}

export function describe(): string {
  return `cts counter=${new Counter().count}`;
}
