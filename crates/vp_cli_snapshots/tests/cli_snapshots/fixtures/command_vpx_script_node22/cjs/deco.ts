// No import or export in a package without "type": CommonJS output, which require()s
// the runtime helpers for the legacy decorator and the private field.
function logged(_target: object, key: string) {
  console.log(`decorated ${key}`);
}
class Service {
  #count = 1;
  @logged
  run() {
    return this.#count;
  }
}
console.log(`commonjs ${typeof module}, count ${new Service().run()}`);
