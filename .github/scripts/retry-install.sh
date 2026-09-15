#!/usr/bin/env bash
# Retry only known external failures. Keep every attempt for CI diagnostics.
# Usage: retry-install.sh <log-prefix> <command> [args...]
set -euo pipefail

log_prefix=$1
shift
mkdir -p "$(dirname "$log_prefix")"

for attempt in 1 2 3; do
  log_file="$log_prefix.$attempt.log"
  set +e
  SFW_JSON_REPORT_PATH="$log_file.json" "$@" 2>&1 | tee "$log_file"
  statuses=("${PIPESTATUS[@]}")
  set -e
  status=${statuses[0]}
  if [ "${statuses[1]}" -ne 0 ]; then
    exit "${statuses[1]}"
  fi
  if [ "$status" -eq 0 ]; then
    # Some sfw internal-error paths return zero after terminating the child.
    if grep -Fq 'Socket Firewall encountered an unexpected error:' "$log_file"; then
      status=1
    else
      exit 0
    fi
  fi

  # Cancellation, package-policy decisions and HTTP client errors must remain
  # failures, even if another request in the same install had a transient error.
  if [ "$status" -eq 130 ] || [ "$status" -eq 143 ] ||
    grep -Eiq 'HTTP[^[:cntrl:]]*\b4[0-9]{2}\b|status (code|client error)[^[:cntrl:]]*\b4[0-9]{2}\b|ERR_PNPM_(TRUST|MINIMUM_RELEASE_AGE)|malicious|policy (violation|rejection)|blocked by' "$log_file"; then
    exit "$status"
  fi

  retry=false
  if [ "${RUNNER_OS:-}" = Windows ] && [ "$status" -eq 127 ] &&
    grep -Fq 'Assertion failed: !(handle->flags & UV_HANDLE_CLOSING)' "$log_file"; then
    # sfw 1.15.1 can abort during Windows process teardown after a successful
    # install. Repeat the complete command; never turn the assertion into success.
    retry=true
  elif grep -Eq 'ECONNRESET|ETIMEDOUT|EAI_AGAIN|HTTPError: Response code 50[234]|HTTP[^[:cntrl:]]*\b50[234]\b' "$log_file"; then
    retry=true
  fi
  if [ "$retry" = false ] || [ "$attempt" -eq 3 ]; then
    exit "$status"
  fi

  echo "::warning::Transient install failure (exit $status); retrying after attempt $attempt/3. Log: $log_file"
  sleep "${CI_INSTALL_RETRY_DELAY_SECONDS:-5}"
done
