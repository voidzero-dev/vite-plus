use napi::bindgen_prelude::{Either, Either3};

pub(super) fn agent_option(
    agent: Vec<String>,
    no_agent: bool,
) -> Option<Either3<bool, String, Vec<String>>> {
    if no_agent {
        Some(Either3::A(false))
    } else if agent.len() == 1 {
        agent.into_iter().next().map(Either3::B)
    } else if agent.is_empty() {
        None
    } else {
        Some(Either3::C(agent))
    }
}

pub(super) fn editor_option(
    mut editor: Vec<String>,
    no_editor: bool,
) -> Option<Either<bool, String>> {
    if no_editor { Some(Either::A(false)) } else { editor.pop().map(Either::B) }
}

pub(super) fn boolean_option(enabled: bool, disabled: bool) -> Option<bool> {
    if disabled { Some(false) } else { enabled.then_some(true) }
}
