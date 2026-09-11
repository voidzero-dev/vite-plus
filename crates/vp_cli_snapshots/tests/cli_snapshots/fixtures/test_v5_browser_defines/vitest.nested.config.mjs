import { project } from './config.mjs';

export default {
  test: {
    name: 'nested',
    projects: [
      './vitest.raw.config.mjs',
      { extends: false, ...project('independent') },
    ],
  },
};
