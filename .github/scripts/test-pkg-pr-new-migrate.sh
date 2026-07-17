#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: .github/scripts/test-pkg-pr-new-migrate.sh <PR-or-SHA> <project-path> [migrate-options...]

Installs an isolated global Vite+ CLI built from a registry bridge commit build
and runs `vp migrate` against a local project. The global CLI and the migrated
project both pin `vite-plus` and `vite` to the matching commit build, resolved
through the registry bridge (https://github.com/voidzero-dev/pkg-pr-registry-bridge)
so they install like ordinary npm versions (0.0.0-commit.<sha>) instead of
mutable pkg.pr.new URLs.

The preview `vp migrate` itself writes the bridge registry into the project's
`.npmrc` (or `.yarnrc.yml` for Yarn Berry), so the migrated project resolves the
commit versions both during this run and in its own CI; this script just
force-stages that file past `.gitignore`.

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

bridge_registry="https://registry-bridge.viteplus.dev/"
bridge_download_base="https://registry-bridge.viteplus.dev/voidzero-dev/vite-plus"

# Commit builds are immutable; PR-number refs are mutable. The bridge's
# pkg.pr.new-style download URL exposes the underlying commit via an
# `x-commit-key: <owner>:<repo>:<sha>` header (HEAD). Resolve the requested ref
# to that 40-char commit so the global install and every dependency spec share
# one immutable key.
resolve_bridge_commit() {
  curl -fsSIL "${bridge_download_base}@${pr_ref}" | tr -d '\r' | awk -F ': ' '
    tolower($1) == "x-commit-key" {
      count = split($2, parts, ":")
      print parts[count]
      exit
    }
  '
}

available_commit="$(resolve_bridge_commit || true)"
case "$available_commit" in
  '' | *[!0-9a-fA-F]*)
    echo "error: could not resolve an immutable registry bridge commit for $pr_ref" >&2
    exit 1
    ;;
esac
if [ "${#available_commit}" -ne 40 ]; then
  echo "error: registry bridge returned an invalid commit for $pr_ref: $available_commit" >&2
  exit 1
fi

resolved_ref="$available_commit"
commit_version="0.0.0-commit.$resolved_ref"
vite_core_spec="npm:@voidzero-dev/vite-plus-core@$commit_version"

# The bridge only serves commit builds published by the preview publish
# workflow (triggered by the `preview-build` label). Fail early with an
# actionable message instead of letting the project install hit an opaque
# registry miss.
if ! curl -fsS "${bridge_registry}@voidzero-dev/vite-plus-core" 2>/dev/null |
  grep -q "0.0.0-commit.$resolved_ref"; then
  echo "error: the registry bridge has no build for commit $resolved_ref" >&2
  echo "Publish it by re-adding the preview-build label to the PR (the preview" >&2
  echo "publish workflow packs, uploads, and registers each labeled commit)." >&2
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
        sub(/^.*@/, "", value)               # pkg.pr.new URL form: keep trailing sha
        sub(/^0\.0\.0-commit\./, "", value)  # registry bridge version form: keep sha
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

  # install.sh installs the global CLI from the registry bridge: the bridge
  # serves the per-platform binaries and resolves the wrapper install to the
  # clearly-defined 0.0.0-commit.<sha> build (no pkg.pr.new URLs).
  echo "Installing Vite+ registry bridge build $resolved_ref (requested $pr_ref) into $pr_home"
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
# The preview `vp migrate` itself writes the bridge registry into the project's
# own `.npmrc` (or `.yarnrc.yml` for Yarn Berry) before it installs, so both this
# run and the project's own CI resolve the commit versions. This script only
# force-stages that file past `.gitignore` below.

hash -r

echo
echo "Using isolated global CLI:"
echo "  requested ref: $pr_ref"
echo "  resolved commit: $resolved_ref"
echo "  executable: $vp_bin"
echo "  installation: $(readlink "$pr_home/current" 2>/dev/null || echo unknown)"
echo "  registry bridge: $bridge_registry (vp migrate writes it into the project)"
echo "  vite-plus spec: $commit_version"
echo "  vite spec: $vite_core_spec"
"$vp_bin" --version

# Remove the existing root lockfile and root node_modules so migrate's reinstall
# resolves from scratch. A stale pre-migrate lockfile can keep an optional-peer
# copy of vite-plus pinned to an older published version (e.g. a nested oxlint's
# `vite-plus: '*'` peer pulled in transitively by vite-plugin-checker/nuxt),
# which the `--no-frozen-lockfile` reinstall preserves rather than deduping,
# leaving `vp why` reporting two vite-plus versions. A clean install lets pnpm
# resolve that optional peer to the in-tree managed version. node_modules is
# gitignored; tracked lockfiles are regenerated by the migrate reinstall.
#
# Only the WORKSPACE ROOT is cleared. Sub-package node_modules are left in place:
# in a pnpm/npm/yarn monorepo they are symlinks/hoisted from the root store and
# the reinstall refreshes them, so wiping them is unnecessary and slow on large
# monorepos.
echo
echo "Clearing root lockfile and node_modules in $project_dir for a clean migrate install"
rm -f "$project_dir/pnpm-lock.yaml" \
  "$project_dir/package-lock.json" \
  "$project_dir/npm-shrinkwrap.json" \
  "$project_dir/yarn.lock" \
  "$project_dir/bun.lock" \
  "$project_dir/bun.lockb"
rm -rf "$project_dir/node_modules"

echo
echo "Running vp migrate in $project_dir"
set +e
(
  # Run migrate through the global CLI's Rust `vp` binary, not by invoking the
  # JS entry (`node dist/bin.js migrate`) directly. `vp`'s migrate routing
  # (delegate_migrate) escalates this preview build over any project-local
  # vite-plus (a preview build always wins) and launches the global CLI's own
  # managed Node, so a project-local vite-plus at the same semver can't take
  # precedence and the project's pinned Node can't block startup. Invoking
  # dist/bin.js directly bypasses that routing and trips the migrate version
  # check. Keep cwd at the project root because project config and plugins may
  # resolve dependencies from process.cwd().
  cd "$project_dir"
  "$vp_bin" migrate "$project_dir" "$@"
)
migrate_status=$?
set -e

if [ "$is_git_repo" -eq 1 ]; then
  # Force-stage the bridge registry config that vp migrate wrote. Projects
  # commonly gitignore .npmrc (and .yarnrc.yml), so without -f it never reaches
  # the project's CI: the commit build then resolves from the default registry,
  # which has no 0.0.0-commit.<sha>, and the supply-chain policy check rejects the
  # lockfile (ERR_PNPM_TARBALL_URL_MISMATCH). Stage whichever file migrate wrote.
  # Only stage config that carries the bridge marker, i.e. was written for this
  # run. A pre-existing ignored `.npmrc`/`.yarnrc.yml` (private-registry auth
  # tokens) that migrate never touched must not land in the migration diff.
  for cfg in .npmrc .yarnrc.yml; do
    if [ -f "$project_dir/$cfg" ] && grep -q "registry-bridge.viteplus.dev" "$project_dir/$cfg" 2>/dev/null; then
      git -C "$project_dir" add -f "$cfg" 2>/dev/null || true
    fi
  done
  echo
  echo "Migration worktree changes (.npmrc force-staged so it survives .gitignore):"
  git -C "$project_dir" status --short
fi

# Show the resolved vite-plus / vite / vitest versions so the result is visible
# at a glance. Each must resolve to exactly ONE version (the commit build for
# vite-plus and vite via the @voidzero-dev/vite-plus-core alias, the bundled
# upstream for vitest); more than one version of any means the migration or
# install is broken. pnpm's `-r` recurses across workspaces and reports all three
# names in one query; npm/yarn/bun `why` accept only a single package, so those
# query one at a time. Detect
# pnpm from the project's own markers (a `packageManager` pin, a workspace file,
# or the post-install lockfile) instead of a `vp env current` + `vp node`
# round-trip: this isolated install runs with VP_NODE_MANAGER=no, so `vp node`
# has no managed Node to parse the JSON and the detection came back empty.
why_recursive=
if grep -qE '"packageManager"[[:space:]]*:[[:space:]]*"pnpm@' "$project_dir/package.json" 2>/dev/null \
  || [ -f "$project_dir/pnpm-workspace.yaml" ] \
  || [ -f "$project_dir/pnpm-lock.yaml" ]; then
  why_recursive=-r
fi
echo
echo "Resolved vite-plus / vite / vitest versions (each should be a single version):"
if [ -n "$why_recursive" ]; then
  # pnpm: one recursive `why` reports all three package names at once.
  (cd "$project_dir" && "$vp_bin" why -r vite-plus vite vitest) || true
else
  # npm/yarn/bun `why` accept only a single package, so query one at a time.
  for pkg in vite-plus vite vitest; do
    (cd "$project_dir" && "$vp_bin" why "$pkg") || true
  done
fi

exit "$migrate_status"
