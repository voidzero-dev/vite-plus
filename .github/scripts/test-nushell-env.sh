#!/usr/bin/env bash

set -euo pipefail

: "${VP_BIN_UNDER_TEST:?Set VP_BIN_UNDER_TEST to the vp binary to test}"

nu_bin="$(command -v "${NU_BIN:-nu}")"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

test_home="$test_root/vp home with spaces"

VP_HOME="$test_home" "$VP_BIN_UNDER_TEST" env setup --refresh >/dev/null

(
  cd "$test_home"
  env -u VP_HOME \
    EXPECTED_VP_HOME="$test_home" \
    PATH="/usr/bin:/bin" \
    "$nu_bin" --commands '
      source env.nu

      let expected_home = ($env.EXPECTED_VP_HOME | path expand --no-symlink)
      if $env.VP_HOME != $expected_home {
        error make {
          msg: $"VP_HOME mismatch: expected ($expected_home), got ($env.VP_HOME)"
        }
      }

      let expected_bin = ($expected_home | path join "bin")
      let actual_bin = ($env.PATH | first)
      if $actual_bin != $expected_bin {
        error make {
          msg: $"PATH mismatch: expected first entry ($expected_bin), got ($actual_bin)"
        }
      }

      let vp_output = (vp --version)
      if $env.LAST_EXIT_CODE != 0 {
        error make {
          msg: "vp --version failed through the Nushell wrapper"
        }
      }
      if ($vp_output | is-empty) {
        error make {
          msg: "vp --version returned no output"
        }
      }
    '
)
