// Shared workspace config (lint/fmt style): must NOT make the root an app target.
export default {
  lint: {},
  plugins: [
    {
      name: 'root-build-input',
      config() {
        return { input: 'root-entry.js' };
      },
    },
  ],
};
