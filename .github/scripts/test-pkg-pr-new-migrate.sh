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
cached_version_dir="$pr_home/pkg-pr-new-$pr_ref"
vp_bin="$pr_home/bin/vp"
vite_plus_package_json="$pr_home/current/node_modules/vite-plus/package.json"
commit_marker="$cached_version_dir/.pkg-pr-new-commit"
pkg_pr_new_base="https://pkg.pr.new/voidzero-dev/vite-plus"
vite_plus_spec="$pkg_pr_new_base@$pr_ref"
vite_plus_core_spec="$pkg_pr_new_base/@voidzero-dev/vite-plus-core@$pr_ref"

resolve_pkg_pr_new_commit() {
  curl -fsSIL "$vite_plus_spec" | tr -d '\r' | awk -F ': ' '
    tolower($1) == "x-commit-key" {
      count = split($2, parts, ":")
      print parts[count]
      exit
    }
  '
}

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

available_commit="$(resolve_pkg_pr_new_commit || true)"
installed_commit="$(read_installed_commit || true)"
current_target="$(readlink "$pr_home/current" 2>/dev/null || true)"
reuse_install=0

if [ -n "$available_commit" ] &&
  [ "$installed_commit" = "$available_commit" ] &&
  [ "$current_target" = "pkg-pr-new-$pr_ref" ] &&
  [ -x "$vp_bin" ] &&
  [ -f "$vite_plus_package_json" ]; then
  reuse_install=1
fi

cleanup() {
  rm -rf "$installer_home"
}
trap cleanup EXIT

if [ "$reuse_install" -eq 1 ]; then
  printf '%s\n' "$available_commit" > "$commit_marker"
  echo "Reusing installed Vite+ pkg.pr.new build $pr_ref ($available_commit) from $pr_home"
else
  if [ -z "$available_commit" ]; then
    echo "Could not verify the current pkg.pr.new commit; reinstalling $pr_ref."
  elif [ -n "$installed_commit" ]; then
    echo "pkg.pr.new build changed: $installed_commit -> $available_commit"
  fi

  # Numeric pkg.pr.new references are mutable PR aliases. If the published
  # commit changed, the reused lockfile can retain the checksum from the older
  # tarball and fail with ERR_PNPM_TARBALL_INTEGRITY. Keep the downloaded
  # runtime/package-manager cache, but force the wrapper dependency to resolve
  # again. Commit SHA references are immutable and use their own cache path.
  case "$pr_ref" in
    *[!0-9]*) ;;
    *)
      rm -rf "$cached_version_dir/node_modules"
      rm -f "$cached_version_dir/pnpm-lock.yaml"
      ;;
  esac

  echo "Installing Vite+ pkg.pr.new build $pr_ref into $pr_home"
  HOME="$installer_home" \
    VP_HOME="$pr_home" \
    VP_PR_VERSION="$pr_ref" \
    VP_NODE_MANAGER=no \
    bash "$installer"

  if [ -n "$available_commit" ]; then
    printf '%s\n' "$available_commit" > "$commit_marker"
  fi
fi

if [ ! -x "$vp_bin" ]; then
  echo "error: installed vp executable not found: $vp_bin" >&2
  exit 1
fi

if [ ! -f "$vite_plus_package_json" ]; then
  echo "error: installed vite-plus package not found: $vite_plus_package_json" >&2
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
export VP_OVERRIDE_PACKAGES="$(printf \
  '{"vite":"%s","vitest":"%s"}' \
  "$vite_plus_core_spec" \
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
