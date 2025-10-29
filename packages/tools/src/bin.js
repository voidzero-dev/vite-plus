import '@oxc-node/core/register';

// defer the import to avoid the register hook is not being called
await import('./index.ts');
