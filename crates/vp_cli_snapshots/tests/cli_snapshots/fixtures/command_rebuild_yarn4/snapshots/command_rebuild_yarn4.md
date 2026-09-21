# command_rebuild_yarn4

## `vp install`

Run the initial build script.

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0007: │ command-rebuild-yarn4@workspace:. must be built because it never has been before or the last one failed
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file builds.txt`

```
built
```

## `vp rebuild`

Yarn Berry rebuilds all packages instead of returning a no-op.

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0007: │ command-rebuild-yarn4@workspace:. must be built because it never has been before or the last one failed
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file builds.txt`

```
built
built
```

## `vp rebuild command-rebuild-yarn4`

Rebuild only the named package.

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0007: │ command-rebuild-yarn4@workspace:. must be built because it never has been before or the last one failed
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file builds.txt`

```
built
built
built
```

## `vp rebuild -- --help`

Forward additional arguments to yarn rebuild.

```
Rebuild the project's native packages

━━━ Usage ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

$ yarn rebuild ...

━━━ Details ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

This command will automatically cause Yarn to forget about previous compilations
of the given packages and to run them again.

Note that while Yarn forgets the compilation, the previous artifacts aren't
erased from the filesystem and may affect the next builds (in good or bad). To
avoid this, you may remove the .yarn/unplugged folder, or any other relevant
location where packages might have been stored (Yarn may offer a way to do that
automatically in the future).

By default all packages will be rebuilt, but you can filter the list by
specifying the names of the packages you want to clear from memory.

━━━ Examples ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Rebuild all packages
  $ yarn rebuild

Rebuild fsevents only
  $ yarn rebuild fsevents
```
