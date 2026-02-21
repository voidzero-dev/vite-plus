# CLI Tips

## Background

As vite-plus grows in features, users often don't discover useful commands, shorter aliases, or relevant options. A lightweight, non-intrusive tip system helps users learn the tool organically.

## Implementation

### Crate Structure

Tips are implemented in `crates/vite_global_cli_tips/`:

```
src/
├── lib.rs          # TipContext, Tip trait, get_tip()
└── tips/
    ├── mod.rs              # Registers all tips
    ├── short_aliases.rs    # Short alias suggestions
    └── use_vpx_or_run.rs   # Unknown command guidance (disabled)
```

### Core Types

```rust
/// Execution context passed to tips
pub struct TipContext {
    pub raw_args: Vec<String>,    // CLI args (excluding program name)
    pub success: bool,            // Command succeeded
    pub unknown_command: bool,    // Command not recognized by CLI
}

/// Trait for implementing tips
pub trait Tip {
    fn matches(&self, ctx: &TipContext) -> bool;
    fn message(&self) -> &'static str;
}
```

### Display

- Tips shown after command output with empty line separator
- Styled with dimmed text using `owo-colors`
- Rate limited: tips display once per 5 minutes (stateless, based on wall clock)
- Disabled in test mode (`VITE_PLUS_CLI_TEST` env var)

```
$ vp list
...

Tip: short aliases available: i (install), rm (remove), un (uninstall), up (update), ls (list), ln (link)
```

### Current Tips

#### ShortAliases

Shown when user runs long-form package manager commands (`install`, `remove`, `uninstall`, `update`, `list`, `link`).

```
Tip: short aliases available: i (install), rm (remove), un (uninstall), up (update), ls (list), ln (link)
```

#### UseVpxOrRun (disabled)

TODO: Enable when `vpx` is supported. Will show for unknown commands.

```
Tip: run a local bin with `vpx <bin>`, or a script with `vp run <script>`
```

### Adding a New Tip

1. Create `src/tips/my_tip.rs`:

```rust
use crate::{Tip, TipContext};

pub struct MyTip;

impl Tip for MyTip {
    fn matches(&self, ctx: &TipContext) -> bool {
        // Return true when tip should be shown
        ctx.is_subcommand("some-command")
    }

    fn message(&self) -> &'static str {
        "Your tip message here"
    }
}
```

2. Register in `src/tips/mod.rs`:

```rust
mod my_tip;
use self::my_tip::MyTip;

pub fn all() -> &'static [&'static dyn Tip] {
    &[&ShortAliases, &UseVpxOrRun, &MyTip]
}
```

## Future Work

- Tip frequency control (random probability, cooldown)
- More contextual tips (feature discovery, guidance)
- AI agent integration (extract tips for AGENTS.md)
