enum Failure { Message = 'mapped failure' }
class Broken {
  constructor(public message: string) {
    throw new Error(message);
  }
}
new Broken(Failure.Message);
