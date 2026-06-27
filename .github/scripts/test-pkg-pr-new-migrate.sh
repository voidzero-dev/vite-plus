#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: .github/scripts/test-pkg-pr-new-migrate.sh <PR-or-SHA> <project-path> [migrate-options...]

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

is_bun_project=0
if [ -f "$project_dir/bun.lock" ] ||
  [ -f "$project_dir/bun.lockb" ] ||
  [ -f "$project_dir/bunfig.toml" ] ||
  node "$script_dir/bun-pkg-pr-new.mjs" is-bun-project "$project_dir/package.json"; then
  is_bun_project=1
fi

repo_root="$(cd "$script_dir/../.." && pwd -P)"
installer="$repo_root/packages/cli/install.sh"
pnpm_version_helper="$script_dir/ensure-pkg-pr-new-pnpm-version.mjs"
override_json_helper="$script_dir/create-pkg-pr-new-overrides.mjs"

if [ ! -f "$installer" ]; then
  echo "error: Vite+ installer not found: $installer" >&2
  exit 2
fi

if [ ! -f "$pnpm_version_helper" ]; then
  echo "error: pnpm version helper not found: $pnpm_version_helper" >&2
  exit 2
fi

if [ ! -f "$override_json_helper" ]; then
  echo "error: pkg.pr.new override helper not found: $override_json_helper" >&2
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

# pnpm 10 before 10.2.0 rewrites pkg.pr.new URL overrides into workspace peer
# declarations, which then fail peer-spec validation. pnpm 11.0.0 through
# 11.8.x can write pkg.pr.new tarball lock entries without integrity metadata,
# which a later frozen install rejects. Upgrade affected package-manager pins
# before migration resolves or invokes pnpm.
node "$pnpm_version_helper" "$project_dir/package.json"

original_home="$HOME"
cache_root="${XDG_CACHE_HOME:-$original_home/.cache}"
pr_home="${VP_PKG_PR_NEW_HOME:-$cache_root/vite-plus/pkg-pr-new/$pr_ref}"
installer_home="$(mktemp -d "${TMPDIR:-/tmp}/vite-plus-pr-installer.XXXXXX")"
pkg_pr_new_base="https://pkg.pr.new/voidzero-dev/vite-plus"
requested_vite_plus_spec="$pkg_pr_new_base@$pr_ref"

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

# PR-number URLs are mutable and pkg.pr.new packages reference their internal
# workspace dependencies by commit SHA. Persisting the PR URL alongside those
# SHA URLs makes package managers install duplicate copies of the same package.
# Resolve once, then use the immutable SHA for the global install and every
# dependency spec written by migration.
resolved_ref="$available_commit"
cached_version_dir="$pr_home/pkg-pr-new-$resolved_ref"
vp_bin="$pr_home/bin/vp"
vite_plus_package_json="$pr_home/current/node_modules/vite-plus/package.json"
global_cli_entry="$pr_home/current/node_modules/vite-plus/dist/bin.js"
commit_marker="$cached_version_dir/.pkg-pr-new-commit"
vite_plus_spec="$pkg_pr_new_base@$resolved_ref"
vite_plus_core_spec="$pkg_pr_new_base/@voidzero-dev/vite-plus-core@$resolved_ref"
vite_override_spec="$vite_plus_core_spec"

if [ "$is_bun_project" -eq 1 ]; then
  bun_repack_script="$script_dir/repack-vite-pr.sh"
  if [ ! -f "$bun_repack_script" ]; then
    echo "error: Bun pkg.pr.new repack helper not found: $bun_repack_script" >&2
    exit 2
  fi

  generated_tarball_path="$(bash "$bun_repack_script" "$resolved_ref" "$project_dir")"
  if [ ! -f "$generated_tarball_path" ]; then
    echo "error: Bun repack script did not create its reported tarball: $generated_tarball_path" >&2
    exit 1
  fi

  # Keep the real Core package directly resolvable alongside the repacked
  # `vite` alias. Bun otherwise nests it under the local tarball dependency.
  node "$script_dir/bun-pkg-pr-new.mjs" \
    add-core-dependency \
    "$project_dir/package.json" \
    "$vite_plus_core_spec"

  # The migrator applies this override to every workspace package. Use an
  # absolute file URL so nested package.json files resolve the same tarball.
  vite_override_spec="file:$generated_tarball_path"
fi

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
export VP_VERSION="$vite_plus_spec"
export VP_OVERRIDE_PACKAGES="$(node \
  "$override_json_helper" \
  "$vite_override_spec" \
  "$vitest_version")"
export VP_FORCE_MIGRATE=1
# pkg.pr.new packages depend on URL-resolved platform binaries. pnpm blocks
# those transitive URL dependencies when blockExoticSubdeps is enabled. The
# migration persists the corresponding workspace setting, while this temporary
# override also lets its pre-rewrite install recover a partially migrated tree.
export PNPM_CONFIG_BLOCK_EXOTIC_SUBDEPS=false
hash -r

echo
echo "Using isolated global CLI:"
echo "  requested ref: $pr_ref"
echo "  resolved commit: $resolved_ref"
echo "  executable: $vp_bin"
echo "  installation: $(readlink "$pr_home/current" 2>/dev/null || echo unknown)"
echo "  vite-plus spec: $VP_VERSION"
echo "  vite spec: $vite_override_spec"
"$vp_bin" --version

if [ "$is_bun_project" -eq 1 ] && [ -d "$project_dir/node_modules" ]; then
  echo
  echo "Removing stale Bun node_modules before migration"
  rm -rf "$project_dir/node_modules"
fi

echo
echo "Running vp migrate in $project_dir"
set +e
(
  # Run the installed JS entry directly so a project-local vite-plus at the
  # same semver cannot take precedence. Keep cwd at the project root because
  # project config and plugins may resolve dependencies from process.cwd().
  cd "$project_dir"
  "$vp_bin" node "$global_cli_entry" migrate "$project_dir" "$@"
)
migrate_status=$?
set -e

if [ "$is_bun_project" -eq 1 ] && [ "$migrate_status" -eq 0 ]; then
  # Migration uses one absolute file URL so every workspace can install the
  # same tarball. Persist portable specs by rebasing that URL relative to each
  # package.json, then refresh Bun's lockfile once with the final paths.
  node "$script_dir/bun-pkg-pr-new.mjs" \
    normalize-vite-paths \
    "$project_dir" \
    "$generated_tarball_path"

  echo
  echo "Reinstalling Bun dependencies with relative Vite tarball paths"
  rm -rf "$project_dir/node_modules"
  set +e
  (
    cd "$project_dir"
    unset VP_OVERRIDE_PACKAGES VP_FORCE_MIGRATE
    "$vp_bin" install
  )
  bun_install_status=$?
  set -e
  if [ "$bun_install_status" -ne 0 ]; then
    echo "error: dependency installation failed after normalizing Bun file paths" >&2
    migrate_status="$bun_install_status"
  fi
fi

if [ "$is_git_repo" -eq 1 ]; then
  echo
  echo "Migration worktree changes:"
  git -C "$project_dir" status --short
  git -C "$project_dir" diff --stat
fi

exit "$migrate_status"
