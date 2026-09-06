#!/bin/bash
# Run locally with bash .github/scripts/test-install-bootstrap.sh; no registry or installed vp needed.
set -eu
cd "$(dirname "$0")/../.."
eval "$(sed '/^main "[$]@"$/d' packages/cli/install.sh)"

test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT
export test_root
mkdir -p "$test_root/package" "$test_root/tmp" "$test_root/scripts"
touch "$test_root/scripts/install.sh"
cat > "$test_root/scripts/install-legacy.sh" <<'LEGACY'
#!/bin/bash
set -eu
INSTALL_DIR="$test_root/data"
SHIM_DIR="$test_root/installed bin"
CACHE_DIR="$test_root/cache"
CONFIG_DIR="$test_root/config"
STATE_DIR="$test_root/state"
case "$1" in /*) ;; *) exit 80 ;; esac
test -f "$1"
printf '%s\n' "$2" "$3" > "$test_root/legacy"
case "$scenario" in
  legacy-failure|piped-legacy-failure) exit 43 ;;
esac
LEGACY
cat > "$test_root/package/vp" <<'BINARY'
#!/bin/bash
if [ "$#" -eq 0 ]; then
  test -z "${VP_SELF_SETUP_SUPPORT_CHECK+x}" || exit 99
  touch "$test_root/binary-invoked"
  if [ "$scenario" = failure ]; then exit 42; fi
  test "${VP_SELF_SETUP_SHELL:-}" = sh || exit 98
  if [ "$scenario" = supported-pr ]; then
    test "$NPM_CONFIG_REGISTRY" = https://registry-bridge.viteplus.dev/ || exit 97
  fi
  printf 'INSTALL_DIR=%q\n' "$test_root/data"
  printf 'SHIM_DIR=%q\n' "$test_root/installed bin"
  printf 'CACHE_DIR=%q\n' "$test_root/cache"
  printf 'CONFIG_DIR=%q\n' "$test_root/config"
  printf 'STATE_DIR=%q\n' "$test_root/state"
  exit 0
fi
test "${VP_SELF_SETUP_SUPPORT_CHECK:-}" = 1 || exit 99
case "$scenario" in
  legacy|legacy-failure|piped-legacy|piped-legacy-failure|pr) printf 'Usage: vp [COMMAND]\n' ;;
  *) printf 'vite-plus-self-setup-v1\n' ;;
esac
BINARY
chmod +x "$test_root/package/vp"
tar czf "$test_root/payload.tgz" -C "$test_root" package
export TMPDIR="$test_root/tmp"
fixture_sha=0123456789012345678901234567890123456789

# Only transport is substituted; extraction, probing, and dispatch run normally.
curl() {
  printf '%s\n' "$*" >> "$test_root/requests"
  case "$*" in
    *file://*) command curl "$@" ;;
    *-fsSIL*) printf 'x-commit-key: voidzero-dev:vite-plus:%s\r\n' "$fixture_sha" ;;
    *'https://custom.example/vite-plus/'*) printf '{"version":"0.2.9"}\n' ;;
    *) cp "$test_root/payload.tgz" "${@: -1}" ;;
  esac
}

for scenario in supported legacy legacy-failure piped-legacy piped-legacy-failure failure pr supported-pr; do
  export scenario
  : > "$test_root/requests"
  rm -f "$test_root/legacy" "$test_root/binary-invoked"
  set +e
  (
    set -e
    VP_VERSION=latest
    LOCAL_TGZ="" LOCAL_BINARY="" PR_VERSION="" PACKAGE_METADATA=""
    NPM_REGISTRY=https://custom.example
    export NPM_CONFIG_REGISTRY="$NPM_REGISTRY"
    INSTALLER_PATH="$test_root/scripts/install.sh"
    if [[ "$scenario" == piped-legacy* ]]; then
      INSTALLER_PATH=""
      LEGACY_INSTALLER_URL="file://$test_root/scripts/install-legacy.sh"
    fi
    if [[ "$scenario" == *pr ]]; then PR_VERSION=2406; fi
    if [ "$scenario" = supported ]; then export VP_SELF_SETUP_SUPPORT_CHECK=original; fi
    main
    test "$NPM_CONFIG_REGISTRY" = https://custom.example
    test "$INSTALL_DIR" = "$test_root/data"
    test "$SHIM_DIR" = "$test_root/installed bin"
    test "$CACHE_DIR" = "$test_root/cache"
    test "$CONFIG_DIR" = "$test_root/config"
    test "$STATE_DIR" = "$test_root/state"
  ) > "$test_root/output" 2>&1
  status=$?
  set -e
  if [ "$scenario" = failure ]; then
    test "$status" -eq 42
  elif [[ "$scenario" == *legacy-failure ]]; then
    test "$status" -eq 43
  elif [ "$status" -ne 0 ]; then
    cat "$test_root/output"
    exit 1
  fi
  case "$scenario" in
    supported|supported-pr|failure)
      test -f "$test_root/binary-invoked"
      test ! -f "$test_root/legacy" ;;
    legacy|legacy-failure|piped-legacy|piped-legacy-failure|pr)
      test -f "$test_root/legacy"
      test ! -f "$test_root/binary-invoked" ;;
  esac
  if [ "$scenario" = pr ]; then
    test "$(head -1 "$test_root/legacy")" = "0.0.0-commit.$fixture_sha"
    test "$(tail -1 "$test_root/legacy")" = 2406
    grep -q "@$fixture_sha -o" "$test_root/requests"
    test "$(wc -l < "$test_root/requests" | tr -d ' ')" = 2
  fi
  test -z "$(ls -A "$test_root/tmp")"
  echo "PASS: $scenario"
done
