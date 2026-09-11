source env.nu

let expected_bin = ($env.EXPECTED_VP_BIN_DIR | path expand --no-symlink)
let env_names = ($env | columns)
for name in [VP_BIN_DIR VP_DATA_DIR VP_CACHE_DIR] {
  if $name in $env_names {
    error make {
      msg: $"env.nu must not export ($name)"
    }
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
