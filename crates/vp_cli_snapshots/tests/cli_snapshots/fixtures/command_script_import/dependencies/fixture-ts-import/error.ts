enum Failure { Value = 'dependency failure' }
export function fail() {
  throw new Error(Failure.Value);
}
