// Kept apart from define-config.ts, which imports Vite at the top level, so a
// command that imports resolve-vite-config.ts but only looks up config files
// (for example `vp create`) does not load Vite at startup.
import { CONFIG_METADATA_ENV } from './utils/constants.ts';

// Number of in-flight `withConfigMetadataResolution` calls, and the marker
// value captured before the first one set it. The marker stays set until the
// LAST overlapping resolution finishes (e.g. several package configs resolved
// concurrently in a monorepo), so an async config that reaches `lazyPlugins`
// while any metadata resolution is still pending is not mistaken for a build.
let configMetadataDepth = 0;
let configMetadataSavedValue: string | undefined;

/**
 * Run a config-metadata resolution (a `resolveConfig` call that loads the
 * user's config purely to read a non-plugin block) with the metadata marker
 * set, so any `lazyPlugins` evaluated during it skips the plugin factory.
 *
 * The marker is scoped in time, not by command name: it is set only while at
 * least one resolution is in flight (ref-counted so overlapping/nested
 * resolutions compose) and the prior value is restored afterwards. By the time
 * the task runner spawns a child (a verbatim `vp run` build, a `vp exec`
 * child), the marker is already gone, so those builds correctly load plugins.
 * Keying on the resolution itself — rather than guessing from the command
 * name — also means command aliases (`vp format`) and not-yet-known commands
 * all load plugins.
 */
export async function withConfigMetadataResolution<T>(fn: () => Promise<T>): Promise<T> {
  if (configMetadataDepth === 0) {
    configMetadataSavedValue = process.env[CONFIG_METADATA_ENV];
    process.env[CONFIG_METADATA_ENV] = '1';
  }
  configMetadataDepth++;
  try {
    return await fn();
  } finally {
    configMetadataDepth--;
    if (configMetadataDepth === 0) {
      if (configMetadataSavedValue === undefined) {
        delete process.env[CONFIG_METADATA_ENV];
      } else {
        process.env[CONFIG_METADATA_ENV] = configMetadataSavedValue;
      }
      configMetadataSavedValue = undefined;
    }
  }
}
