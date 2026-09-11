interface TestProjectView {
  config: {
    browser?: { enabled?: boolean };
    defines?: Record<string, unknown>;
  };
}

interface VitestConfigureContext {
  vitest: {
    version: string;
    injectTestProject: (config: unknown) => Promise<TestProjectView[]>;
  };
  project: TestProjectView;
}

const guardedInstances = new WeakSet<object>();

function clearBrowserDefines(project: TestProjectView) {
  if (project.config.browser?.enabled) {
    // Replace the map, rather than clearing it in place: it can be shared
    // with the Vite config or a Node project that still needs its defines.
    project.config.defines = {};
  }
}

/**
 * Temporary backport of https://github.com/vitest-dev/vitest/pull/11198.
 * Vitest 5.0.0 overwrites the browser globals initialized by Vite with
 * JSON-encoded define expressions. Leave initialization to Vite instead.
 * Do not decode expressions here or change Node projects' runtime defines.
 *
 * Register in bundled Vite, not defineConfig: raw configs and independent or
 * referenced projects must receive the fix too. configureVitest runs for each
 * project after resolution and before its config is sent to the browser.
 *
 * Remove this module, its build injection, and backport-only unit tests when the
 * pinned Vitest release includes #11198. The build only includes it for 5.0.0;
 * the runtime guard leaves other Vitest versions unchanged. Keep the browser regressions.
 */
export function vitestBrowserDefinesBackportPlugin() {
  return {
    name: 'vite-plus:vitest-browser-defines-backport',
    configureVitest: {
      // Install the injection guard before user hooks capture injectTestProjects.
      order: 'pre',
      handler({ vitest, project }: VitestConfigureContext) {
        if (vitest.version !== '5.0.0') {
          return;
        }
        clearBrowserDefines(project);

        // Vitest 5.0.0 does not call configureVitest for projects added through
        // injectTestProject. Cover that API too, without wrapping it once per
        // initial project or changing the returned projects and error behavior.
        if (guardedInstances.has(vitest)) {
          return;
        }
        guardedInstances.add(vitest);
        const injectTestProject = vitest.injectTestProject.bind(vitest);
        vitest.injectTestProject = async (config) => {
          const projects = await injectTestProject(config);
          projects.forEach(clearBrowserDefines);
          return projects;
        };
      },
    },
  };
}
