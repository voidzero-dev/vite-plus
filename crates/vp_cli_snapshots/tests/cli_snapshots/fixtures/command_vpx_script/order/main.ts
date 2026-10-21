import { from as withExtension } from './dep.js';
import { from as extensionless } from './dep';

console.log('./dep.js ->', withExtension, '| ./dep ->', extensionless);
