#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: .github/scripts/test-pkg-pr-new-migrate.sh <PR-or-SHA> <project-path> [migrate-options...]

Installs an isolated global Vite+ CLI built from a pkg.pr.new commit and runs
`vp migrate` against a local project. The migrated project pins `vite-plus` and
`vite` to the matching commit build, resolved through the pkg.pr.new registry
bridge (https://github.com/fengmk2/pkg-pr-registry-bridge) so they install like
ordinary npm versions (0.0.0-commit.<sha>) instead of mutable pkg.pr.new URLs.

Persists the bridge registry into the project's `.npmrc` (npm/pnpm/Yarn
Classic/Bun) and, for Yarn Berry projects, `.yarnrc.yml`, so the migrated
project resolves the commit versions both during this run and in its own CI.

Examples:
  .github/scripts/test-pkg-pr-new-migrate.sh 1891 /path/to/npmx.dev
  .github/scripts/test-pkg-pr-new-migrate.sh 4eb2104c /path/to/project --no-interactive

Environment variables:
  VP_PKG_PR_NEW_HOME  Override the isolated global CLI installation directory.
  ALLOW_DIRTY=1       Allow migration in a dirty Git worktree.
EOF
}

if [ "$#" -lt 2 ]; then
  usage >&2
  exit 2
fi

pr_ref="$1"
project_input="$2"
shift 2

case "$pr_ref" in
  '' | *[![:alnum:]._-]*)
    echo "error: PR or SHA contains unsupported characters: $pr_ref" >&2
    exit 2
    ;;
esac

if [ ! -d "$project_input" ]; then
  echo "error: project directory does not exist: $project_input" >&2
  exit 2
fi

project_dir="$(cd "$project_input" && pwd -P)"
if [ ! -f "$project_dir/package.json" ]; then
  echo "error: package.json not found in project: $project_dir" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/../.." && pwd -P)"
installer="$repo_root/packages/cli/install.sh"

if [ ! -f "$installer" ]; then
  echo "error: Vite+ installer not found: $installer" >&2
  exit 2
fi

is_git_repo=0
if command -v git >/dev/null 2>&1 && git -C "$project_dir" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  is_git_repo=1
  if [ "${ALLOW_DIRTY:-0}" != "1" ] && [ -n "$(git -C "$project_dir" status --porcelain)" ]; then
    echo "error: project worktree is dirty: $project_dir" >&2
    echo "Commit or stash its changes, or rerun with ALLOW_DIRTY=1." >&2
    exit 2
  fi
fi

bridge_registry="https://pkg-pr-registry-bridge.render.vip/"
pkg_pr_new_base="https://pkg.pr.new/voidzero-dev/vite-plus"
requested_vite_plus_spec="$pkg_pr_new_base@$pr_ref"

# pkg.pr.new commit builds are immutable; PR-number URLs are mutable and the
# registry bridge only mirrors commit builds. Resolve the requested PR or SHA to
# its underlying 40-char commit so the global install and every dependency spec
# share one immutable key.
resolve_pkg_pr_new_commit() {
  curl -fsSIL "$requested_vite_plus_spec" | tr -d '\r' | awk -F ': ' '
    tolower($1) == "x-commit-key" {
      count = split($2, parts, ":")
      print parts[count]
      exit
    }
  '
}

available_commit="$(resolve_pkg_pr_new_commit || true)"
case "$available_commit" in
  '' | *[!0-9a-fA-F]*)
    echo "error: could not resolve an immutable pkg.pr.new commit for $pr_ref" >&2
    exit 1
    ;;
esac
if [ "${#available_commit}" -ne 40 ]; then
  echo "error: pkg.pr.new returned an invalid commit for $pr_ref: $available_commit" >&2
  exit 1
fi

resolved_ref="$available_commit"
commit_version="0.0.0-commit.$resolved_ref"
vite_core_spec="npm:@voidzero-dev/vite-plus-core@$commit_version"

# The bridge only serves commit builds it has been told about (registered by the
# pkg.pr.new publish workflow). Fail early with an actionable message instead of
# letting the project install hit an opaque registry miss.
if ! curl -fsS "${bridge_registry}@voidzero-dev/vite-plus-core" 2>/dev/null |
  grep -q "0.0.0-commit.$resolved_ref"; then
  echo "error: the registry bridge has no build for commit $resolved_ref" >&2
  echo "Ensure the pkg.pr.new publish workflow registered it, or register it manually:" >&2
  echo "  curl -fsS -X POST -H \"authorization: Bearer \$PKG_PR_BRIDGE_ADMIN_TOKEN\" \\" >&2
  echo "    -H 'content-type: application/json' -d '{\"ref\":\"commit.$resolved_ref\"}' \\" >&2
  echo "    ${bridge_registry}-/refs" >&2
  exit 1
fi

original_home="$HOME"
cache_root="${XDG_CACHE_HOME:-$original_home/.cache}"
pr_home="${VP_PKG_PR_NEW_HOME:-$cache_root/vite-plus/pkg-pr-new/$pr_ref}"
installer_home="$(mktemp -d "${TMPDIR:-/tmp}/vite-plus-pr-installer.XXXXXX")"

cached_version_dir="$pr_home/pkg-pr-new-$resolved_ref"
vp_bin="$pr_home/bin/vp"
vite_plus_package_json="$pr_home/current/node_modules/vite-plus/package.json"
global_cli_entry="$pr_home/current/node_modules/vite-plus/dist/bin.js"
commit_marker="$cached_version_dir/.pkg-pr-new-commit"

read_installed_commit() {
  if [ -f "$commit_marker" ]; then
    head -n 1 "$commit_marker"
    return
  fi

  if [ -f "$vite_plus_package_json" ]; then
    awk -F '"' '
      $2 == "@voidzero-dev/vite-plus-core" {
        value = $4
        sub(/^.*@/, "", value)
        print value
        exit
      }
    ' "$vite_plus_package_json"
  fi
}

installed_commit="$(read_installed_commit || true)"
current_target="$(readlink "$pr_home/current" 2>/dev/null || true)"
reuse_install=0

if [ "$installed_commit" = "$resolved_ref" ] &&
  [ "$current_target" = "pkg-pr-new-$resolved_ref" ] &&
  [ -x "$vp_bin" ] &&
  [ -f "$vite_plus_package_json" ] &&
  [ -f "$global_cli_entry" ]; then
  reuse_install=1
fi

cleanup() {
  rm -rf "$installer_home"
}
trap cleanup EXIT

if [ "$reuse_install" -eq 1 ]; then
  printf '%s\n' "$resolved_ref" > "$commit_marker"
  echo "Reusing installed Vite+ pkg.pr.new build $resolved_ref (requested $pr_ref) from $pr_home"
else
  if [ -n "$installed_commit" ] && [ "$installed_commit" != "$resolved_ref" ]; then
    echo "pkg.pr.new build changed: $installed_commit -> $resolved_ref"
  elif [ -n "$installed_commit" ]; then
    echo "Reinstalling pkg.pr.new build $resolved_ref with an immutable cache key"
  fi

  # This helper owns a dedicated VP_HOME for each requested PR/ref. Remember
  # the previous immutable install so it can be removed only after the new one
  # succeeds, while retaining shared runtime and package-manager caches.
  previous_target=""
  if [ -n "$current_target" ] && [ "$current_target" != "pkg-pr-new-$resolved_ref" ]; then
    case "$current_target" in
      pkg-pr-new-*) previous_target="$current_target" ;;
    esac
  fi

  # The global CLI ships per-platform binaries that the bridge cannot serve
  # through npm's tarball path, so install it straight from pkg.pr.new by its
  # immutable commit.
  echo "Installing Vite+ pkg.pr.new build $resolved_ref (requested $pr_ref) into $pr_home"
  HOME="$installer_home" \
    VP_HOME="$pr_home" \
    VP_PR_VERSION="$resolved_ref" \
    VP_NODE_MANAGER=no \
    bash "$installer"

  if [ -n "$previous_target" ]; then
    rm -rf "$pr_home/$previous_target"
  fi
  printf '%s\n' "$resolved_ref" > "$commit_marker"
fi

if [ ! -x "$vp_bin" ]; then
  echo "error: installed vp executable not found: $vp_bin" >&2
  exit 1
fi

if [ ! -f "$vite_plus_package_json" ]; then
  echo "error: installed vite-plus package not found: $vite_plus_package_json" >&2
  exit 1
fi

if [ ! -f "$global_cli_entry" ]; then
  echo "error: installed Vite+ CLI entry not found: $global_cli_entry" >&2
  exit 1
fi

vitest_version="$(awk -F '"' '$2 == "vitest" { print $4; exit }' "$vite_plus_package_json")"
if [ -z "$vitest_version" ]; then
  echo "error: could not determine the bundled Vitest version from $vite_plus_package_json" >&2
  exit 1
fi

export VP_HOME="$pr_home"
export PATH="$VP_HOME/bin:$PATH"
# vite-plus and vite (-> vite-plus-core) become ordinary npm versions resolved
# through the bridge, so the normal upgrade path re-pins them (no force-override
# needed). The values are constrained (commit SHA, semver) so the override JSON
# needs no escaping.
export VP_VERSION="$commit_version"
export VP_OVERRIDE_PACKAGES="{\"vite\":\"$vite_core_spec\",\"vitest\":\"$vitest_version\"}"
# Point every package manager at the registry bridge. It serves the vite-plus /
# vite-plus-core / per-platform CLI commit builds and proxies everything else to
# npmjs, so the project resolves the commit versions like any released package.
# Yarn Berry only honors YARN_NPM_REGISTRY_SERVER; Bun honors npm_config_registry.
export npm_config_registry="$bridge_registry"
export YARN_NPM_REGISTRY_SERVER="$bridge_registry"

# Persist the bridge registry into the project's own config files so the
# migrated project installs the commit builds in ITS OWN CI too, not just during
# this run (the env vars above are not persisted). pnpm in particular resolves
# from .npmrc, not npm_config_registry, and without this fetches the commit
# version from registry.npmjs.org and fails with ERR_PNPM_NO_MATCHING_VERSION.
registry_marker="# pkg.pr.new registry bridge (added by test-pkg-pr-new-migrate.sh)"

# .npmrc is read by npm, pnpm, Yarn Classic and Bun.
project_npmrc="$project_dir/.npmrc"
if ! grep -qsF "$registry_marker" "$project_npmrc"; then
  if [ -s "$project_npmrc" ]; then
    printf '\n' >> "$project_npmrc"
  fi
  printf '%s\nregistry=%s\n' "$registry_marker" "$bridge_registry" >> "$project_npmrc"
fi

# Yarn Berry ignores .npmrc and reads .yarnrc.yml instead. The migration's own
# .yarnrc.yml rewrite preserves unrelated keys, so npmRegistryServer survives.
project_yarnrc="$project_dir/.yarnrc.yml"
is_yarn_berry=0
if [ -f "$project_yarnrc" ] ||
  { [ -f "$project_dir/yarn.lock" ] && grep -q '^__metadata:' "$project_dir/yarn.lock" 2>/dev/null; } ||
  grep -qE '"packageManager"[[:space:]]*:[[:space:]]*"yarn@([2-9]|[1-9][0-9])' "$project_dir/package.json" 2>/dev/null; then
  is_yarn_berry=1
fi
if [ "$is_yarn_berry" -eq 1 ]; then
  if grep -qsE '^npmRegistryServer:' "$project_yarnrc"; then
    # Override an existing default-registry setting in place.
    sed -i.pkg-pr-new.bak -E \
      "s|^npmRegistryServer:.*|npmRegistryServer: \"$bridge_registry\"|" "$project_yarnrc"
    rm -f "$project_yarnrc.pkg-pr-new.bak"
  elif ! grep -qsF "$registry_marker" "$project_yarnrc"; then
    if [ -s "$project_yarnrc" ]; then
      printf '\n' >> "$project_yarnrc"
    fi
    printf '%s\nnpmRegistryServer: "%s"\n' "$registry_marker" "$bridge_registry" >> "$project_yarnrc"
  fi
fi

hash -r

echo
echo "Using isolated global CLI:"
echo "  requested ref: $pr_ref"
echo "  resolved commit: $resolved_ref"
echo "  executable: $vp_bin"
echo "  installation: $(readlink "$pr_home/current" 2>/dev/null || echo unknown)"
echo "  registry bridge: $bridge_registry"
echo "  project .npmrc: $project_npmrc"
echo "  vite-plus spec: $commit_version"
echo "  vite spec: $vite_core_spec"
"$vp_bin" --version

# Resolve the preview CLI's own managed Node, independent of the target
# project's pin. Probe from the isolated VP_HOME (no project .node-version) so
# we get the global default rather than the project's. A project pinned to an
# old/unsupported Node would otherwise fail to launch the preview dist/bin.js,
# even though the isolated CLI ships a compatible runtime.
cli_node_version="$(cd "$pr_home" && "$vp_bin" --version 2>/dev/null \
  | sed -nE 's/.*Node\.js[[:space:]]+v?([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)"
echo "  cli node: ${cli_node_version:-unknown}"

echo
echo "Running vp migrate in $project_dir"
set +e
(
  # Run the installed JS entry directly so a project-local vite-plus at the
  # same semver cannot take precedence. Keep cwd at the project root because
  # project config and plugins may resolve dependencies from process.cwd().
  # Pin the CLI's own Node (via `env exec --node`) so the project's pinned Node
  # version cannot block dist/bin.js from starting.
  cd "$project_dir"
  if [ -n "$cli_node_version" ]; then
    "$vp_bin" env exec --node "$cli_node_version" node "$global_cli_entry" migrate "$project_dir" "$@"
  else
    "$vp_bin" node "$global_cli_entry" migrate "$project_dir" "$@"
  fi
)
migrate_status=$?
set -e

if [ "$is_git_repo" -eq 1 ]; then
  echo
  echo "Migration worktree changes:"
  git -C "$project_dir" status --short
  git -C "$project_dir" diff --stat
fi

exit "$migrate_status"
