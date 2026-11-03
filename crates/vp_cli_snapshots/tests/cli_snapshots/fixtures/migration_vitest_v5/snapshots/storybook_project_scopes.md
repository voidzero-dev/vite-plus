# storybook_project_scopes

Use Storybook story patterns to keep jsdom text assertions unchanged without ownership reviews, while still migrating browser assertions.

## `vpt rm example.test.ts`


## `vpt write-file vite.config.ts 'import path from '\''node:path'\'';
import { fileURLToPath } from '\''node:url'\'';
import { storybookTest as stories } from '\''@storybook/addon-vitest/vitest-plugin'\'';
const dirname = path.dirname(fileURLToPath(import.meta.url));
export default { test: { projects: [
  { extends: true, test: { name: '\''integration'\'', environment: '\''jsdom'\'', include: ['\''src/**/*.test.tsx'\''], setupFiles: ['\''./setup.ts'\''] } },
  { extends: true, plugins: [stories({ configDir: path.join(dirname, '\''.storybook-test'\'') })], test: { name: '\''component-browser'\'', browser: { enabled: true } } },
] } };
'`


## `vpt write-file .storybook-test/main.ts 'const config = { stories: ['\''../src/routes/**/*.stories.tsx'\''] };
export default config;
'`


## `vpt write-file setup.ts 'import '\''@testing-library/jest-dom/vitest'\'';
'`


## `vpt write-file src/account.test.tsx 'import { expect } from '\''vitest'\'';
export function checkAccount(element) {
  expect(element).toHaveTextContent('\''Could not save your display name.'\'');
  expect(element).toHaveTextContent('\''Could not save your display name.'\'');
  expect(element).toHaveTextContent('\''Could not load profile information.'\'');
}
'`


## `vpt write-file src/routes/account.stories.tsx 'import { expect } from '\''vitest'\'';
export function checkBrowser(element) { expect(element).toHaveTextContent('\''partial'\''); }
'`


## `vpt write-file src/unrelated.js 'export function checkOtherRunner(element) { expect(element).toHaveTextContent('\''partial'\''); }
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file src/account.test.tsx src/routes/account.stories.tsx src/unrelated.js .storybook-test/main.ts`

```
import { expect } from 'vite-plus/test';
export function checkAccount(element) {
  expect(element).toHaveTextContent('Could not save your display name.');
  expect(element).toHaveTextContent('Could not save your display name.');
  expect(element).toHaveTextContent('Could not load profile information.');
}
import { expect } from 'vite-plus/test';
export function checkBrowser(element) { expect(element).toMatchTextContent('partial'); }
export function checkOtherRunner(element) { expect(element).toHaveTextContent('partial'); }
const config = { stories: ['../src/routes/**/*.stories.tsx'] };
export default config;
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
