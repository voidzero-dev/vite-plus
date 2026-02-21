//! Tip suggesting vpx or vp run for unknown commands.

use super::{Tip, TipContext};

/// Suggest `vpx <bin>` or `vp run <script>` when an unknown command is used.
pub struct UseVpxOrRun;

impl Tip for UseVpxOrRun {
    fn matches(&self, _ctx: &TipContext) -> bool {
        // TODO: Enable when `vpx` is supported
        // ctx.is_unknown_command_error()
        false
    }

    fn message(&self) -> &'static str {
        "Run a local binary with `vpx <bin>`, or a script with `vp run <script>`"
    }
}

// TODO: Re-enable tests when `vpx` is supported
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tips::tip_context_from_command;
//
//     #[test]
//     fn matches_on_unknown_command() {
//         let ctx = tip_context_from_command("vp typecheck");
//         assert!(UseVpxOrRun.matches(&ctx));
//         assert!(ctx.is_unknown_command_error());
//     }
//
//     #[test]
//     fn does_not_match_on_known_command() {
//         let ctx = tip_context_from_command("vp build");
//         assert!(!UseVpxOrRun.matches(&ctx));
//         assert!(!ctx.is_unknown_command_error());
//     }
// }
