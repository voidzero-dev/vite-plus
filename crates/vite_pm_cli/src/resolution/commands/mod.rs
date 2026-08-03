mod add;
mod approve_builds;
mod audit;
mod cache;
mod config;
mod dedupe;
mod deprecate;
mod dist_tag;
mod dlx;
mod fund;
mod install;
mod link;
mod list;
mod login;
mod logout;
mod outdated;
mod owner;
mod pack;
mod ping;
mod prune;
mod publish;
mod rebuild;
mod remove;
mod search;
mod stage;
mod token;
mod unlink;
mod update;
mod version;
mod view;
mod whoami;
mod why;

pub use add::AddArgs;
pub(crate) use add::SaveDependencyArgs;
pub use approve_builds::ApproveBuildsArgs;
pub use audit::AuditArgs;
pub use cache::CacheArgs;
pub use config::ConfigCommand;
pub use dedupe::DedupeArgs;
pub use deprecate::DeprecateArgs;
pub use dist_tag::DistTagCommand;
pub use dlx::DlxArgs;
pub use fund::FundArgs;
pub use install::InstallArgs;
pub use link::LinkArgs;
pub use list::ListArgs;
pub use login::LoginArgs;
pub use logout::LogoutArgs;
pub use outdated::{OutdatedArgs, OutdatedFormat};
pub use owner::OwnerCommand;
pub use pack::PackArgs;
pub use ping::PingArgs;
pub use prune::PruneArgs;
pub use publish::PublishArgs;
pub use rebuild::RebuildArgs;
pub use remove::RemoveArgs;
pub use search::SearchArgs;
pub use stage::StageCommand;
pub use token::TokenCommand;
pub use unlink::UnlinkArgs;
pub use update::UpdateArgs;
pub use version::VersionArgs;
pub use view::ViewArgs;
pub use whoami::WhoamiArgs;
pub use why::WhyArgs;

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(value) if value > 0 => Ok(value),
        Ok(_) => Err("value must be at least 1".to_string()),
        Err(error) => Err(error.to_string()),
    }
}
