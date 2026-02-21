//! Tip suggesting short aliases for long-form commands.

use super::{Tip, TipContext};

/// Long-form commands that have short aliases.
const LONG_FORMS: &[&str] = &["install", "remove", "uninstall", "update", "list", "link"];

/// Suggest short aliases when user runs a long-form command.
pub struct ShortAliases;

impl Tip for ShortAliases {
    fn matches(&self, ctx: &TipContext) -> bool {
        ctx.subcommand().is_some_and(|cmd| LONG_FORMS.contains(&cmd))
    }

    fn message(&self) -> &'static str {
        "Available short aliases: i = install, rm = remove, un = uninstall, up = update, ls = list, ln = link"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tips::tip_context_from_command;

    #[test]
    fn matches_long_form_commands() {
        for cmd in LONG_FORMS {
            let ctx = tip_context_from_command(&format!("vp {cmd}"));
            assert!(ShortAliases.matches(&ctx), "should match {cmd}");
        }
    }

    #[test]
    fn does_not_match_short_form_commands() {
        let short_forms = ["i", "rm", "un", "up", "ln"];
        for cmd in short_forms {
            let ctx = tip_context_from_command(&format!("vp {cmd}"));
            assert!(!ShortAliases.matches(&ctx), "should not match {cmd}");
        }
    }

    #[test]
    fn does_not_match_other_commands() {
        let other_commands = ["build", "test", "lint", "run", "pack"];
        for cmd in other_commands {
            let ctx = tip_context_from_command(&format!("vp {cmd}"));
            assert!(!ShortAliases.matches(&ctx), "should not match {cmd}");
        }
    }

    #[test]
    fn install_shows_short_alias_tip() {
        let ctx = tip_context_from_command("vp install");
        assert!(ShortAliases.matches(&ctx));
    }

    #[test]
    fn short_form_does_not_show_tip() {
        let ctx = tip_context_from_command("vp i");
        assert!(!ShortAliases.matches(&ctx));
    }
}
