#!/bin/bash
# Run from the repository root with RUNNER_TEMP and TEST_VERSION set.
set -eo pipefail

run_case() {
  mode="$1"
  expected="$2"
  case_dir="$RUNNER_TEMP/vite-plus-provenance-sh-$mode"
  port_file="$case_dir/port"
  log_file="$case_dir/requests.jsonl"
  vp_home="$case_dir/vp-home"

  rm -rf "$case_dir"
  mkdir -p "$case_dir/home" "$vp_home"
  node packages/cli/tests/fixtures/provenance-registry.mjs \
    --port-file "$port_file" \
    --log-file "$log_file" \
    --mode "$mode" \
    --version "$TEST_VERSION" &
  server_pid=$!

  for _ in $(seq 1 100); do
    [ -s "$port_file" ] && break
    sleep 0.1
  done
  if [ ! -s "$port_file" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    echo "Mock registry did not start"
    return 1
  fi

  registry="http://127.0.0.1:$(cat "$port_file")"
  set +e
  output=$(env \
    CI=true \
    HOME="$case_dir/home" \
    VP_HOME="$vp_home" \
    VP_NODE_MANAGER=no \
    VP_VERSION="$TEST_VERSION" \
    NPM_CONFIG_REGISTRY="$registry" \
    bash packages/cli/install.sh 2>&1)
  status=$?
  set -e

  kill "$server_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
  printf '%s\n' "$output"

  if [ "$status" -eq 0 ]; then
    echo "Expected the fixture tarball endpoint to prevent installation"
    return 1
  fi

  if [ "$expected" = reject ]; then
    printf '%s\n' "$output" | grep -F \
      "does not contain supported npm provenance metadata"
    printf '%s\n' "$output" | grep -F \
      "@voidzero-dev/vite-plus-cli-"
    printf '%s\n' "$output" | grep -F "$TEST_VERSION"

    if grep -F '"path":"/platform.tgz"' "$log_file"; then
      echo "Platform tarball was requested before provenance validation"
      return 1
    fi
    if [ -e "$vp_home/current" ] || [ -e "$vp_home/$TEST_VERSION/bin/vp" ]; then
      echo "Rejected package left an active or executable installation"
      return 1
    fi
  else
    if printf '%s\n' "$output" | grep -F \
      "does not contain supported npm provenance metadata"; then
      echo "Supported provenance metadata was rejected"
      return 1
    fi
    grep -F '"path":"/platform.tgz"' "$log_file"
  fi
}

run_case missing reject
run_case malformed reject
run_case top-level-only reject
run_case dotted-top-level-key reject
run_case unsupported reject
run_case valid-v1 allow
run_case valid-v0.2 allow

# Commit previews can omit provenance on any registry. Other
# prereleases and malformed commit versions must still be rejected.
TEST_VERSION=0.0.0-commit.0123456789abcdef0123456789abcdef01234567
run_case missing allow
run_case unsupported allow
TEST_VERSION=0.0.0-commit.0123456789ABCDEF0123456789ABCDEF01234567
run_case missing allow
for TEST_VERSION in \
  1.2.3-beta.1 \
  0.0.0 \
  0.0.0-beta.1 \
  0.0.0-commit. \
  0.0.0-commit.abc1234 \
  0.0.0-commit.0123456789abcdef0123456789abcdef012345678 \
  0.0.0-commit.0123456789abcdef0123456789abcdef0123456g \
  0.0.0-COMMIT.0123456789abcdef0123456789abcdef01234567 \
  0.0.0-commit.0123456789abcdef0123456789abcdef01234567.extra; do
  run_case missing reject
done
