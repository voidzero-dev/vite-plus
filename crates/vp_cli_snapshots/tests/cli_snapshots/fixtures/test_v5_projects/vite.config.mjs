import { defineConfig, defineProject } from 'vite-plus';

const project = (name, extra = {}) => ({
  ...extra,
  test: { name, include: ['identity.test.js'], provide: { value: name } },
});
const options = {
  single: {},
  shared: { projects: [project('default'), project('inherited', { extends: true }), project('independent', { extends: false })] },
  separate: { sharedViteServer: false, projects: [project('default'), project('independent', { extends: false })] },
  composed: { projects: [defineProject(project('composed'))] },
  referenced: { projects: ['./vitest.referenced.config.js'] },
  nested: { projects: ['./vitest.nested.config.js'] },
};

export default defineConfig({
  plugins: [{
    name: 'test-root-hook',
    configureServer(server) { globalThis.__vpHookServers.push(server); },
    configureVitest({ project }) { globalThis.__vpHookProjects.push(project.name); },
  }],
  test: {
    include: ['identity.test.js'],
    provide: { value: 'root' },
    ...options[process.env.VP_TEST_PROJECT_CASE],
  },
});
