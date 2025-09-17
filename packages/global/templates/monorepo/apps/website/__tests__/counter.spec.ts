import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setupCounter } from '../src/counter.ts';

describe('setupCounter', () => {
  let button: HTMLButtonElement;

  beforeEach(() => {
    // Create a button element for testing
    button = document.createElement('button');
    button.id = 'counter';
    document.body.appendChild(button);
  });

  afterEach(() => {
    // Clean up the DOM after each test
    document.body.removeChild(button);
  });

  it('should initialize counter with 0', () => {
    setupCounter(button);
    expect(button.innerHTML).toBe('count is 0');
  });

  it('should increment counter on click', () => {
    setupCounter(button);

    // Initial state
    expect(button.innerHTML).toBe('count is 0');

    // First click
    button.click();
    expect(button.innerHTML).toBe('count is 1');

    // Second click
    button.click();
    expect(button.innerHTML).toBe('count is 2');
  });

  it('should handle multiple clicks correctly', () => {
    setupCounter(button);

    // Click 5 times
    for (let i = 0; i < 5; i++) {
      button.click();
    }

    expect(button.innerHTML).toBe('count is 5');
  });
});
