//! Tip suggesting vpx or vp run for unknown commands.

use super::{Tip, TipContext};

/// Suggest `vpx <pkg>` or `vp run <script>` when an unknown command is used.
pub struct UseVpxOrRun;

impl Tip for UseVpxOrRun {
    fn matches(&self, ctx: &TipContext) -> bool {
        ctx.is_unknown_command_error()
    }

    fn message(&self) -> &'static str {
        "Execute a package binary with `vpx <pkg[@version]>`, or a script with `vp run <script>`"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tips::tip_context_from_command;

    #[test]
    fn matches_on_unknown_command() {
        let ctx = tip_context_from_command("vp typecheck");
        assert!(UseVpxOrRun.matches(&ctx));
        assert!(ctx.is_unknown_command_error());
    }

    #[test]
    fn does_not_match_on_known_command() {
        let ctx = tip_context_from_command("vp build");
        assert!(!UseVpxOrRun.matches(&ctx));
        assert!(!ctx.is_unknown_command_error());
    }
}
