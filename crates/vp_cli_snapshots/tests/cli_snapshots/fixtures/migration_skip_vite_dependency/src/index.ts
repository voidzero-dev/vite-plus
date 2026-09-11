import { defineConfig, type Plugin } from 'vite';
import { describe, it, expect } from 'vitest';

export function myApp(): Plugin {
  return {
    name: 'my-app',
    configResolved(config) {
      console.log(config);
    },
  };
}

describe('myApp', () => {
  it('should work', () => {
    expect(myApp()).toBeDefined();
  });
});

export default defineConfig({
  plugins: [myApp()],
});
