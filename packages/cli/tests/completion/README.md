# Completion integration tests

After building the JavaScript packages, run `pnpm test:completion`. It builds `vp_global_cli` and uses Cargo's reported executable. To use a prebuilt binary, set `VP_COMPLETION_CONSUMER_BIN` to its exact path; an invalid path fails the suite.

CI runs this suite separately with the downloaded native binary. Ordinary `pnpm test:unit` does not build Rust or discover the `*.integration.ts` files.
