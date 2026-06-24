// Run a programmatic Vite build, mirroring a downstream framework CLI
// (e.g. `node framework-cli.js build`) that is spawned underneath `vp exec`.
// `vp exec` does not load the config for metadata, so the config-metadata
// marker is unset here; `lazyPlugins` must load the plugins for the build to
// produce a usable index.html.
import { build } from 'vite-plus';

await build({ root: import.meta.dirname, logLevel: 'silent' });
