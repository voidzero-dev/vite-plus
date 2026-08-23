mod config;
mod create;
mod hooks;
mod migrate;
mod parse;
mod staged;

pub(crate) fn command_specs() -> [&'static usage_rs::spec::Spec<'static>; 5] {
    [config::spec(), create::spec(), hooks::spec(), migrate::spec(), staged::spec()]
}
