import colors from 'picocolors';

import { VERSION } from './constants';

export function createBanner() {
  return `${colors.bold('VITE')} v${VERSION}`;
}
