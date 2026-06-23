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

original_home="$HOME"
cache_root="${XDG_CACHE_HOME:-$original_home/.cache}"
pr_home="${VP_PKG_PR_NEW_HOME:-$cache_root/vite-plus/pkg-pr-new/$pr_ref}"
installer_home="$(mktemp -d "${TMPDIR:-/tmp}/vite-plus-pr-installer.XXXXXX")"

cleanup() {
  rm -rf "$installer_home"
}
trap cleanup EXIT

echo "Installing Vite+ pkg.pr.new build $pr_ref into $pr_home"
HOME="$installer_home" \
  VP_HOME="$pr_home" \
  VP_PR_VERSION="$pr_ref" \
  VP_NODE_MANAGER=no \
  bash "$installer"

vp_bin="$pr_home/bin/vp"
if [ ! -x "$vp_bin" ]; then
  echo "error: installed vp executable not found: $vp_bin" >&2
  exit 1
fi

vite_plus_package_json="$pr_home/current/node_modules/vite-plus/package.json"
if [ ! -f "$vite_plus_package_json" ]; then
  echo "error: installed vite-plus package not found: $vite_plus_package_json" >&2
  exit 1
fi

vitest_version="$(awk -F '"' '$2 == "vitest" { print $4; exit }' "$vite_plus_package_json")"
if [ -z "$vitest_version" ]; then
  echo "error: could not determine the bundled Vitest version from $vite_plus_package_json" >&2
  exit 1
fi

pkg_pr_new_base="https://pkg.pr.new/voidzero-dev/vite-plus"
vite_plus_spec="$pkg_pr_new_base@$pr_ref"
vite_plus_core_spec="$pkg_pr_new_base/@voidzero-dev/vite-plus-core@$pr_ref"

export VP_HOME="$pr_home"
export PATH="$VP_HOME/bin:$PATH"
export VP_VERSION="$vite_plus_spec"
export VP_OVERRIDE_PACKAGES="$(printf \
  '{"vite":"%s","vitest":"%s"}' \
  "$vite_plus_core_spec" \
  "$vitest_version")"
export VP_FORCE_MIGRATE=1
hash -r

echo
echo "Using isolated global CLI:"
echo "  executable: $vp_bin"
echo "  installation: $(readlink "$pr_home/current" 2>/dev/null || echo unknown)"
echo "  vite-plus spec: $VP_VERSION"
echo "  vite spec: $vite_plus_core_spec"
"$vp_bin" --version

echo
echo "Running vp migrate in $project_dir"
runner_dir="$installer_home/runner"
mkdir -p "$runner_dir"
set +e
(
  # Resolve the CLI from an empty directory so a project-local vite-plus at the
  # same semver cannot take precedence over the installed pkg.pr.new build.
  cd "$runner_dir"
  "$vp_bin" migrate "$project_dir" "$@"
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
