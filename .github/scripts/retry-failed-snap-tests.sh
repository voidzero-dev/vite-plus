#!/usr/bin/env bash
# Retry only the snap test cases whose snap.txt changed, up to max_retries times.
# Usage: retry-failed-snap-tests.sh [max_retries]
set -euo pipefail

max_retries=${1:-2}

for retry in $(seq 1 "$max_retries"); do
  changed=$(git diff --name-only -- 'packages/cli/snap-tests*/*/snap.txt')
  if [ -z "$changed" ]; then
    exit 0
  fi

  echo "::warning::Snapshot diff detected, retry $retry/$max_retries for failed cases..."
  git diff --stat -- 'packages/cli/snap-tests*/*/snap.txt'

  failed_local=$(echo "$changed" | grep -v 'snap-tests-global/' | sed -E 's|packages/cli/snap-tests/([^/]+)/.*|\1|' | sort -u || true)
  failed_global=$(echo "$changed" | grep 'snap-tests-global/' | sed -E 's|packages/cli/snap-tests-global/([^/]+)/.*|\1|' | sort -u || true)

  echo "$changed" | xargs git checkout --

  for name in $failed_local; do
    echo "Retrying local snap test: $name"
    RUST_BACKTRACE=1 pnpm -F ./packages/cli snap-test-local "$name"
  done
  for name in $failed_global; do
    echo "Retrying global snap test: $name"
    RUST_BACKTRACE=1 pnpm -F ./packages/cli snap-test-global "$name"
  done
done

# Final check after all retries
if ! git diff --quiet -- 'packages/cli/snap-tests*/*/snap.txt'; then
  echo "::error::Snapshot diff detected after $max_retries retries. Run 'pnpm -F vite-plus snap-test' locally and commit the updated snap.txt files."
  git diff --stat
  git diff
  exit 1
fi
