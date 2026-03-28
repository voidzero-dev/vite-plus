use super::*;

pub(super) fn unsupported_publish_protocols(
    package_manager: &PackageManager,
    summary: DependencyProtocolSummary,
) -> Vec<&'static str> {
    // Publish-time protocol rewriting differs across package managers, so release stays
    // conservative and only allows protocols that the selected native publisher documents.
    // npm workspaces: https://docs.npmjs.com/cli/v11/using-npm/workspaces/
    // pnpm workspaces/catalogs: https://pnpm.io/workspaces / https://pnpm.io/catalogs
    // Yarn workspace protocol: https://yarnpkg.com/protocol/workspace
    // Bun workspaces/catalogs: https://bun.sh/docs/pm/workspaces / https://bun.sh/docs/pm/catalogs
    let mut protocols = Vec::new();

    if summary.workspace && !supports_publish_rewrite(package_manager) {
        protocols.push("workspace:");
    }
    if summary.catalog && !supports_publish_rewrite(package_manager) {
        protocols.push("catalog:");
    }
    if summary.file {
        protocols.push("file:");
    }
    if summary.link {
        protocols.push("link:");
    }
    if summary.portal {
        protocols.push("portal:");
    }
    if summary.patch {
        protocols.push("patch:");
    }
    if summary.jsr {
        protocols.push("jsr:");
    }

    protocols
}

fn supports_publish_rewrite(package_manager: &PackageManager) -> bool {
    match package_manager.client {
        PackageManagerType::Pnpm | PackageManagerType::Bun => true,
        PackageManagerType::Yarn => !package_manager.version.starts_with("1."),
        PackageManagerType::Npm => false,
    }
}
