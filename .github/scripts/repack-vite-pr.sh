#!/usr/bin/env bash

set -euo pipefail

pr_ref="${1:-1891}"
project_input="${2:-$PWD}"

case "$pr_ref" in
  '' | *[![:alnum:]._-]*)
    echo "error: PR or commit contains unsupported characters: $pr_ref" >&2
    exit 2
    ;;
esac

if [ ! -d "$project_input" ]; then
  echo "error: project directory does not exist: $project_input" >&2
  exit 2
fi

project_dir="$(cd "$project_input" && pwd -P)"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
output_path="$project_dir/vendor/vite-plus-core-as-vite-$pr_ref.tgz"
core_url="https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@$pr_ref"
vite_plus_url="https://pkg.pr.new/voidzero-dev/vite-plus/vite-plus@$pr_ref"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/vite-plus-core-as-vite.XXXXXX")"

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

curl -fsSL "$core_url" -o "$tmp_dir/vite-plus-core.tgz"
mkdir -p "$tmp_dir/unpacked"
tar -xzf "$tmp_dir/vite-plus-core.tgz" -C "$tmp_dir/unpacked"

package_json="$tmp_dir/unpacked/package/package.json"
if [ ! -f "$package_json" ]; then
  echo "error: downloaded package does not contain package/package.json" >&2
  exit 1
fi

node "$script_dir/bun-pkg-pr-new.mjs" patch-package "$package_json" "$core_url" "$vite_plus_url"

mkdir -p "$(dirname "$output_path")"
tar -czf "$output_path" -C "$tmp_dir/unpacked" package

printf '%s\n' "$output_path"
