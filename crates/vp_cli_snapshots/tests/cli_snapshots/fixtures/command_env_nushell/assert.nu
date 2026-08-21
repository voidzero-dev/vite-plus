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

let bin_count = ($env.PATH | where { $in == $expected_bin } | length)
if $bin_count != 1 {
  error make {
    msg: $"PATH contains the Vite+ bin directory ($bin_count) times"
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

vp env use 20.18.0 --no-install
if ("VP_NODE_VERSION" not-in $env) {
  error make {
    msg: "vp env use did not set VP_NODE_VERSION"
  }
}
if $env.VP_NODE_VERSION != "20.18.0" {
  error make {
    msg: $"VP_NODE_VERSION mismatch: expected 20.18.0, got ($env.VP_NODE_VERSION)"
  }
}

vp env use --unset
if ("VP_NODE_VERSION" in $env) {
  error make {
    msg: "vp env use --unset did not remove VP_NODE_VERSION"
  }
}

vp env use --no-install
if ("VP_NODE_VERSION" not-in $env) {
  error make {
    msg: "vp env use without a version did not set VP_NODE_VERSION"
  }
}
if $env.VP_NODE_VERSION != "22.18.0" {
  error make {
    msg: $"file-based VP_NODE_VERSION mismatch: expected 22.18.0, got ($env.VP_NODE_VERSION)"
  }
}

print "Nushell environment checks passed"
