import { configFor } from './config.mjs';

// Deliberately raw: the CLI backport must not depend on defineConfig.
export default configFor('raw');
