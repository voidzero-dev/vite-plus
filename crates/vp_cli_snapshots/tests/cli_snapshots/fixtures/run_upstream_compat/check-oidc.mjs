import assert from 'node:assert/strict';

assert.equal(process.env.ACTIONS_ID_TOKEN_REQUEST_URL, 'https://example.invalid/oidc');
assert.equal(process.env.ACTIONS_ID_TOKEN_REQUEST_TOKEN, 'test-token-1');
console.log('OIDC request variables are available');
