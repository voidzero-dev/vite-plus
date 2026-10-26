import variant from '#variant';
import image from './logo.svg';
import { order } from './order.js';
import generated from './generated.cjs';

enum Mode { Test = 'typescript' }
console.log(JSON.stringify({ mode: Mode.Test, variant, image, order, generated }));
