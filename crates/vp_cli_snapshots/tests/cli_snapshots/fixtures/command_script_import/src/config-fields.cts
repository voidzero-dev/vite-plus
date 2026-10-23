let writes: number = 0;
class Base {
  set field(_value: number) { writes += 1; }
}
class Derived extends Base {
  field = 1;
}
new Derived();
module.exports = { writes };
