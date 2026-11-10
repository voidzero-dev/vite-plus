function logged(value: unknown) {
  return value;
}

@logged
class Service {}

console.log(new Service());
