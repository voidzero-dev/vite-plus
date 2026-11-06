import { value } from '@value';
enum Result { Value = 'fork' }
process.send!(`${Result.Value}: ${value}`, () => process.disconnect());
