//! Publish-protocol compatibility checks.
//!
//! Some dependency protocols are safe to keep in a monorepo but unsafe to ship to the registry
//! unless the selected package manager rewrites them during publish. This module centralizes that
//! compatibility matrix so the release flow can reject unsafe combinations before any publish
//! attempt begins.

use super::*;

/// Returns dependency protocols that are unsafe for the selected publisher to ship as-is.
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

/// Returns whether the package manager is known to rewrite workspace/catalog-style references at
/// publish time.
fn supports_publish_rewrite(package_manager: &PackageManager) -> bool {
    match package_manager.client {
        PackageManagerType::Pnpm | PackageManagerType::Bun => true,
        PackageManagerType::Yarn => !package_manager.version.starts_with("1."),
        PackageManagerType::Npm => false,
    }
}
