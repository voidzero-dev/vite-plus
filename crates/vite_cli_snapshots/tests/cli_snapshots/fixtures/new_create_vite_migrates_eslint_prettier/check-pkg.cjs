// Prints the migrated package.json facts the snapshot asserts on: the lint
// script rewrite, the ESLint dev dependency removal, and the added vite-plus
// dev dependency.
const p = require('./my-react-ts/package.json');
console.log('lint:', p.scripts && p.scripts.lint);
console.log('eslint dep:', !!(p.devDependencies && p.devDependencies.eslint));
console.log('vite-plus dep:', !!(p.devDependencies && p.devDependencies['vite-plus']));
