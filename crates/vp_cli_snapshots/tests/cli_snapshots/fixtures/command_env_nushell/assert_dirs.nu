source env.nu

let expected_bin = ($env.EXPECTED_VP_BIN_DIR | path expand --no-symlink)
if $env.VP_BIN_DIR != $expected_bin {
  error make {
    msg: $"VP_BIN_DIR mismatch: expected ($expected_bin), got ($env.VP_BIN_DIR)"
  }
}

let expected_data = ($env.EXPECTED_VP_DATA_DIR | path expand --no-symlink)
if $env.VP_DATA_DIR != $expected_data {
  error make {
    msg: $"VP_DATA_DIR mismatch: expected ($expected_data), got ($env.VP_DATA_DIR)"
  }
}

let expected_cache = ($env.EXPECTED_VP_CACHE_DIR | path expand --no-symlink)
if $env.VP_CACHE_DIR != $expected_cache {
  error make {
    msg: $"VP_CACHE_DIR mismatch: expected ($expected_cache), got ($env.VP_CACHE_DIR)"
  }
}

let actual_bin = ($env.PATH | first)
if $actual_bin != $expected_bin {
  error make {
    msg: $"PATH mismatch: expected first entry ($expected_bin), got ($actual_bin)"
  }
}

let bin_count = ($env.PATH | where { $in == $expected_bin } | length)
if $bin_count != 1 {
  error make {
    msg: $"PATH contains the Vite+ bin directory ($bin_count) times"
  }
}

print "Nushell metacharacter path checks passed"
