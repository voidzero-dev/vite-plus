//! Low-Node passthrough threshold and version comparison.
//!
//! The threshold mirrors `packages/cli/package.json#engines.node`
//! (`"^20.19.0 || ^22.18.0 || >=24.11.0"`). Bump both together.

/// Minimum supported Node.js version for the full Vite+ CLI.
/// Below this, eligible commands degrade to passthrough mode.
/// KEEP IN SYNC with packages/cli/package.json#engines.node.
pub const MIN_SUPPORTED_NODE: &str = "20.19.0";

/// The full engines.node constraint. Used to detect unsupported ranges like
/// Node 21.x, 22.0–22.17, 23.x, 24.0–24.10 that pass the simple floor check
/// but are not actually supported.
/// KEEP IN SYNC with packages/cli/package.json#engines.node.
pub const SUPPORTED_NODE_RANGE: &str = "^20.19.0 || ^22.18.0 || >=24.11.0";

/// Returns true when passthrough should activate for the given version string.
///
/// Handles two cases:
/// 1. **Exact version** (e.g. "14.15.0", "v22.18.0"): triggers if below
///    `MIN_SUPPORTED_NODE`.
/// 2. **Range string** from `engines.node` (e.g. ">=14",
///    "^20.19.0 || ^22.18.0 || >=24.11.0"): triggers if the range does NOT
///    satisfy `SUPPORTED_NODE_RANGE`, meaning the project targets an
///    unsupported Node version.
///
/// Invalid inputs return false (do not trigger passthrough).
#[must_use]
pub fn is_node_below_min(version: &str) -> bool {
    let trimmed = version.trim_start_matches('v');

    // Case 1: exact version — simple floor check.
    if let Ok(actual) = node_semver::Version::parse(trimmed) {
        let Ok(min) = node_semver::Version::parse(MIN_SUPPORTED_NODE) else {
            return false;
        };
        return actual < min;
    }

    // Case 2: range string from engines.node — check if ANY representative
    // supported version satisfies the project's range. If none do, the project
    // targets an unsupported Node version and passthrough should trigger.
    //
    // We test three boundary versions that cover all supported ranges:
    //   ^20.19.0 → 20.19.0,  ^22.18.0 → 22.18.0,  >=24.11.0 → 24.11.0
    if let Ok(range) = node_semver::Range::parse(trimmed) {
        let any_supported = [MIN_SUPPORTED_NODE, "22.18.0", "24.11.0"]
            .iter()
            .filter_map(|v| node_semver::Version::parse(v).ok())
            .any(|v| range.satisfies(&v));
        return !any_supported;
    }

    // Unparseable: do not trigger.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_min_triggers() {
        assert!(is_node_below_min("14.15.0"));
        assert!(is_node_below_min("18.20.0"));
        assert!(is_node_below_min("20.18.0"));
    }

    #[test]
    fn at_or_above_min_does_not_trigger() {
        // boundary: exactly min is NOT below
        assert!(!is_node_below_min("20.19.0"));
        assert!(!is_node_below_min("20.19.1"));
        assert!(!is_node_below_min("22.18.0"));
        assert!(!is_node_below_min("24.11.0"));
    }

    #[test]
    fn v_prefix_tolerated() {
        assert!(is_node_below_min("v14.15.0"));
    }

    #[test]
    fn invalid_version_does_not_trigger() {
        assert!(!is_node_below_min("not-a-version"));
        assert!(!is_node_below_min(""));
    }

    #[test]
    fn range_below_min_triggers() {
        // engines.node targets old Node only — 20.19.0 does NOT satisfy these ranges
        assert!(is_node_below_min("^18.0.0"));
        assert!(is_node_below_min("^14.0.0 || ^16.0.0 || ^18.0.0"));
        assert!(is_node_below_min(">=14.0.0 <20.0.0"));
    }

    #[test]
    fn range_matching_supported_does_not_trigger() {
        // engines.node matches the Vite+ supported range
        assert!(!is_node_below_min("^20.19.0 || ^22.18.0 || >=24.11.0"));
        assert!(!is_node_below_min(">=20.19.0"));
        // >=14 includes 20.19.0, so the project *could* run on a supported version
        assert!(!is_node_below_min(">=14"));
        assert!(!is_node_below_min(">=14.0.0"));
    }

    #[test]
    fn range_partially_overlapping_triggers() {
        // Node 21.x is in the range but NOT in supported set
        assert!(is_node_below_min("^21.0.0"));
    }

    #[test]
    fn range_partially_overlapping_with_supported_does_not_trigger() {
        // ^22.0.0 includes 22.18.0 which IS supported → no passthrough
        assert!(!is_node_below_min("^22.0.0"));
        // Mixed range: ^18 excludes 20.x, but ^22 includes 22.18.0 → no passthrough
        assert!(!is_node_below_min("^18.0.0 || ^22.0.0"));
    }
}
