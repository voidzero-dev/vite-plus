// A CommonJS `.cts`: TypeScript's `export =` compiles to `module.exports`.
class Counter {
  count = 1;
}

function describe(): string {
  return `cts counter=${new Counter().count}`;
}

export = { describe };
