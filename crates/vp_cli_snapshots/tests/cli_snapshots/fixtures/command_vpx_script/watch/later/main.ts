const later = await import('./later.js').then(
  (module: { value: string }) => module.value,
  () => 'missing',
);
console.log('later', later);
