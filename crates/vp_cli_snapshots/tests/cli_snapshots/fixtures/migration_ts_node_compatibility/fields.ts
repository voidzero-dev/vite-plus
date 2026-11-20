class Base {
  value = 'base';
}
class Derived extends Base {
  value: string;
}
console.log(new Derived().value);
