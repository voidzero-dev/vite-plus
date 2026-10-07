// Preloaded with `--require`, which runs before the loader's `--import`.
enum Mode {
  Required = 'required preload ran',
}
(globalThis as { setupMode?: string }).setupMode = Mode.Required;
