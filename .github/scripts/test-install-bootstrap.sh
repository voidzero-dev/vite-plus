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
touch "$test_root/binary-probed"
case "$scenario" in
  legacy|legacy-failure|piped-legacy|piped-legacy-failure|pr) printf 'Usage: vp [COMMAND]\n' ;;
  *) printf 'vite-plus-self-setup-v1\n' ;;
esac
BINARY
chmod +x "$test_root/package/vp"
tar czf "$test_root/payload.tgz" -C "$test_root" package
fixture_integrity="sha512-$(openssl dgst -sha512 -binary "$test_root/payload.tgz" | openssl base64 -A)"
touch "$test_root/package/tampered"
tar czf "$test_root/tampered.tgz" -C "$test_root" package
rm "$test_root/package/tampered"
export TMPDIR="$test_root/tmp"
fixture_sha=0123456789012345678901234567890123456789

# Record extraction so integrity failures must stop before tar sees the archive.
tar() {
  touch "$test_root/extracted"
  command tar "$@"
}

# Only transport is substituted; extraction, probing, and dispatch run normally.
curl() {
  printf '%s\n' "$*" >> "$test_root/requests"
  case "$*" in
    *file://*) command curl "$@" ;;
    *-fsSIL*) printf 'x-commit-key: voidzero-dev:vite-plus:%s\r\n' "$fixture_sha" ;;
    *'https://custom.example/vite-plus/'*) printf '{"version":"0.2.9"}\n' ;;
    *'/@voidzero-dev%2Fvite-plus-cli-'*)
      # Release payloads must pass the real provenance gate before handoff.
      local integrity_field="\"integrity\":\"$fixture_integrity\","
      case "$scenario" in
        integrity-missing*) integrity_field="" ;;
        integrity-malformed) integrity_field='"integrity":"sha512-invalid",' ;;
        integrity-unsupported) integrity_field="\"integrity\":\"sha256-${fixture_integrity#sha512-}\"," ;;
        integrity-noncanonical) integrity_field="\"integrity\":\"${fixture_integrity%???}B==\"," ;;
        integrity-wrong-type) integrity_field="\"integrity\":[\"$fixture_integrity\"]," ;;
        integrity-dotted-key) integrity_field="\"dist.integrity\":\"$fixture_integrity\"," ;;
        integrity-duplicate) integrity_field="$integrity_field$integrity_field" ;;
      esac
      local attestations='"attestations":{"provenance":{"predicateType":"https://slsa.dev/provenance/v1"}},'
      if [[ "$scenario" == *pr ]]; then attestations=""; fi
      printf '{"dist":{%s%s"tarball":"https://custom.example/platform.tgz"}}\n' "$integrity_field" "$attestations" ;;
    *)
      local payload=payload
      if [[ "$scenario" == integrity-mismatch* ]]; then payload=tampered; fi
      cp "$test_root/$payload.tgz" "${@: -1}" ;;
  esac
}

for scenario in supported legacy legacy-failure piped-legacy piped-legacy-failure failure pr supported-pr \
  integrity-missing integrity-malformed integrity-unsupported integrity-noncanonical integrity-wrong-type \
  integrity-dotted-key integrity-duplicate integrity-mismatch integrity-missing-pr integrity-mismatch-pr; do
  export scenario
  : > "$test_root/requests"
  rm -f "$test_root/legacy" "$test_root/binary-invoked" "$test_root/binary-probed" "$test_root/extracted"
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
  if [[ "$scenario" == integrity-* ]]; then
    test "$status" -ne 0
    case "$scenario" in
      integrity-mismatch*)
        grep -q 'Platform package integrity mismatch' "$test_root/output"
        grep -q 'platform.tgz -o' "$test_root/requests" ;;
      integrity-duplicate) grep -q 'Failed to parse CLI package metadata' "$test_root/output" ;;
      *) grep -q 'does not include a valid SHA-512 dist.integrity' "$test_root/output" ;;
    esac
    if [[ "$scenario" != integrity-mismatch* ]]; then
      ! grep -q 'platform.tgz -o' "$test_root/requests"
    fi
    test ! -f "$test_root/extracted"
    test ! -f "$test_root/binary-probed"
    test ! -f "$test_root/binary-invoked"
    test ! -f "$test_root/legacy"
  elif [ "$scenario" = failure ]; then
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
    grep -q "https://registry-bridge.viteplus.dev/@voidzero-dev%2Fvite-plus-cli-.*/0.0.0-commit.$fixture_sha" "$test_root/requests"
    test "$(wc -l < "$test_root/requests" | tr -d ' ')" = 3
  fi
  test -z "$(ls -A "$test_root/tmp")"
  echo "PASS: $scenario"
done

# Exercise every available hash tool, including macOS and minimal Linux fallbacks.
for hash_tool in sha512sum shasum openssl; do
  command -v "$hash_tool" > /dev/null || continue
  (
    command() {
      if [ "$1" = -v ]; then
        case "$2" in
          sha512sum|shasum|openssl) [ "$2" = "$hash_tool" ] || return 1 ;;
        esac
      fi
      builtin command "$@"
    }
    verify_archive_integrity "$test_root/payload.tgz" "$fixture_integrity"
    if verify_archive_integrity "$test_root/tampered.tgz" "$fixture_integrity" > "$test_root/output" 2>&1; then
      echo "$hash_tool accepted a modified archive"
      exit 1
    fi
    grep -q 'Platform package integrity mismatch' "$test_root/output"
  )
  echo "PASS: $hash_tool verification"
done

(
  sha512sum() { return 1; }
  if verify_archive_integrity "$test_root/payload.tgz" "$fixture_integrity" > "$test_root/output" 2>&1; then
    echo 'Hash command failure was ignored'
    exit 1
  fi
  grep -q 'Failed to hash platform package' "$test_root/output"
)
echo 'PASS: hash command failure'

(
  base64() { return 1; }
  if verify_archive_integrity "$test_root/payload.tgz" "$fixture_integrity" > "$test_root/output" 2>&1; then
    echo 'Base64 command failure was ignored'
    exit 1
  fi
  grep -q 'Failed to decode platform package integrity' "$test_root/output"
)
echo 'PASS: base64 command failure'

(
  command() {
    if [ "$1" = -v ]; then
      case "$2" in sha512sum|shasum|openssl) return 1 ;; esac
    fi
    builtin command "$@"
  }
  if verify_archive_integrity "$test_root/payload.tgz" "$fixture_integrity" > "$test_root/output" 2>&1; then
    echo 'Missing hash tools were ignored'
    exit 1
  fi
  grep -q 'SHA-512 verification requires sha512sum, shasum, or openssl' "$test_root/output"
)
echo 'PASS: missing hash tools'
