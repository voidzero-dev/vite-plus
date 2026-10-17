// Syntax Node.js type stripping rejects: enums, namespaces, parameter
// properties, and legacy decorators. The class field is lowered to a runtime
// helper that must resolve from vite-plus, not from this project.
enum Color {
  Red = 'red',
  Green = 'green',
}

namespace Config {
  export const retries = 3;
}

function logged(_target: object, key: string) {
  console.log(`decorated ${key}`);
}

class Point {
  count = 1;
  #secret = 'private field';
  constructor(
    private readonly x: number,
    public y: number,
  ) {}

  @logged
  sum(): number {
    return this.x + this.y + this.count;
  }

  reveal(): string {
    return this.#secret;
  }
}

const point = new Point(1, 2);
console.log(Color.Green, Config.retries, point.sum(), point.reveal());
