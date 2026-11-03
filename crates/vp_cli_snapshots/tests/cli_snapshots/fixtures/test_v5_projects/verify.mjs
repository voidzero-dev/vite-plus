import assert from 'node:assert/strict';
import { createVitest } from 'vite-plus/test/node';

const integrationPlugins = [
  'vite-plus:vitest-resolver',
  'vite-plus:auto-inline-matcher-deps',
  'vite-plus:coverage-version-guard',
];
for (const name of ['single', 'shared', 'separate', 'composed', 'referenced', 'nested']) {
  process.env.VP_TEST_PROJECT_CASE = name;
  globalThis.__vpHookServers = [];
  globalThis.__vpHookProjects = [];
  const runner = await createVitest({ config: './vite.config.mjs', watch: false, reporters: [] });
  try {
    for (const active of runner.projects) {
      for (const pluginName of integrationPlugins) {
        assert.equal(active.viteConfig.plugins.filter((plugin) => plugin.name === pluginName).length, 1, `${name}/${active.name}/${pluginName}`);
      }
    }
    const servers = new Set(runner.projects.map((active) => active.vite));
    assert.equal(globalThis.__vpHookServers.length, new Set(globalThis.__vpHookServers).size);
    assert.deepEqual(globalThis.__vpHookProjects.toSorted(), runner.projects.filter((project) => project.viteConfig.plugins.some((plugin) => plugin.name === 'test-root-hook')).map((project) => project.name).toSorted());
    if (name === 'shared') {
      assert.equal(servers.size, 2);
      assert.equal(runner.getProjectByName('default').vite, runner.getProjectByName('inherited').vite);
      assert.notEqual(runner.getProjectByName('default').vite, runner.getProjectByName('independent').vite);
      assert.equal(runner.getProjectByName('independent').viteConfig.plugins.some((plugin) => plugin.name === 'test-root-hook'), false);
    }
    if (name === 'separate') {
      assert.equal(servers.size, 2);
      assert.notEqual(runner.projects[0].vite, runner.projects[1].vite);
      assert.equal(runner.getProjectByName('independent').viteConfig.plugins.some((plugin) => plugin.name === 'test-root-hook'), false);
    }
    const result = await runner.start();
    assert.equal(result.unhandledErrors.length, 0, JSON.stringify(result.unhandledErrors));
    assert.equal(runner.state.getCountOfFailedTests(), 0, JSON.stringify(runner.state.getFiles().map((file) => ({ project: file.projectName, errors: file.result?.errors, tests: file.tasks.map((task) => task.result?.errors) }))));
    assert.equal(runner.state.getFiles().length, runner.projects.length);
    console.log(`${name}: ${runner.projects.length} projects, ${servers.size} servers; imports, hooks, and assertions passed`);
  } finally {
    await runner.close();
  }
}
