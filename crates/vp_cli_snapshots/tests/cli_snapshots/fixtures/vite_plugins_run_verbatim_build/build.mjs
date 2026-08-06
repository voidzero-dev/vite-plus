// A verbatim (non-`vp`) package.json task that runs a programmatic Vite build.
// `vp run build` dispatches this as a Verbatim child. The config-metadata
// marker is cleared once `vp run`'s task discovery finishes, so by the time
// this build runs the marker is unset and `lazyPlugins` must load the user's
// plugins, otherwise it silently builds without them.
import { build } from 'vite-plus';

await build({ root: import.meta.dirname, logLevel: 'silent' });
