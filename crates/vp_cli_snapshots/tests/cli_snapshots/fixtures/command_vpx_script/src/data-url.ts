const mod = await import('data:text/javascript,export const value = 7');
console.log('data: URL import:', mod.value);
