import cliPkg from '../../package.json' with { type: 'json' };

// Initialize only in CLI entry points, so library imports leave the host's
// environment alone. A new invocation advertises the version executing it,
// including when the global CLI delegates to a different local version.
export function setUserAgent(): void {
  process.env.VP_USER_AGENT = `vp/${cliPkg.version}`;
}
