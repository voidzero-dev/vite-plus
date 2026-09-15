def main [mode: string] {
  if $mode not-in ["config", "parent"] {
    error make {msg: "Expected config or parent mode"}
  }
  $env.HOME = ($env.PWD | path join "user")
  $env.XDG_CONFIG_HOME = ($env.HOME | path join "config")
  $env.XDG_DATA_DIRS = ($env.HOME | path join "system-data")
  $env.VP_HOME = ($env.PWD | path join "installation")
  $env.VP_SKIP_DEPS_INSTALL = "1"
  $env.VP_VERSION = "nushell-test"
  $env.VP_NODE_MANAGER = "no"
  $env.VP_PM_MANAGER = "no"
  $env.CI = "true"
  hide-env -i XDG_DATA_HOME

  let config_dir = ($env.XDG_CONFIG_HOME | path join "nushell")
  let custom_data = ($env.HOME | path join "custom-data")
  # Use a non-default directory on both Linux and macOS so the mismatch is observable.
  mkdir $config_dir
  '' | save ($config_dir | path join "env.nu")
  if $mode == "config" {
    '$env.XDG_DATA_HOME = ($env.HOME | path join "custom-data")'
      | save ($config_dir | path join "config.nu")
  } else {
    $env.XDG_DATA_HOME = $custom_data
    '' | save ($config_dir | path join "config.nu")
  }

  # Real interactive startup loads config.nu before invoking the standalone installer.
  let install = (^$nu.current-exe --no-history --execute 'try { source install.nu } catch { |err| print -e $err; exit 1 }; exit 0' | complete)
  if $install.exit_code != 0 {
    error make {msg: $"Installer session failed: ($install.stdout) ($install.stderr)"}
  }
  let installed = ($install.stdout | from json)
  let session_dir = $installed.directory
  if $installed.data_home != $custom_data {
    error make {msg: "The installer session did not inherit the configured XDG_DATA_HOME"}
  }
  let snippet = ($custom_data | path join "nushell/vendor/autoload/vite-plus.nu")
  if not ($snippet | path exists) {
    error make {msg: "Installer did not write vite-plus.nu into the child-resolved directory"}
  }
  if not ($env.VP_HOME | path join "current/bin/.vp-setup-complete" | path exists) {
    error make {msg: "Installer did not complete setup"}
  }

  # Launch from the original parent environment, not from the modified interactive session.
  let fresh = (^$nu.current-exe --no-history --execute 'try { source probe.nu } catch { |err| print -e $err; exit 1 }; exit 0' | complete)
  if $fresh.exit_code != 0 {
    error make {msg: $"Fresh session failed: ($fresh.stdout) ($fresh.stderr)"}
  }
  let state = ($fresh.stdout | from json)
  if $state.data_home != $custom_data {
    error make {msg: "The fresh session did not load the configured XDG_DATA_HOME"}
  }
  let expected_loaded = ($mode == "parent")
  if $state.directory != $session_dir {
    error make {msg: "Fresh session did not resolve the original session's autoload directory"}
  }
  if (($snippet | path dirname) == $session_dir) != $expected_loaded {
    error make {msg: "Unexpected agreement between installer and session autoload directories"}
  }
  if $state.loaded != $expected_loaded {
    error make {msg: $"Unexpected Vite+ autoload state: ($state.loaded)"}
  }
  print "Standalone setup completed and wrote vite-plus.nu"
  print "Both sessions have the configured XDG_DATA_HOME after startup: true"
  print $"Installer and session directories match: ($expected_loaded)"
  print $"Fresh session loaded Vite+ environment: ($state.loaded)"

  if $mode == "config" {
    # Apply the documented workaround without changing the XDG settings.
    # source requires a parse-time path, so write a quoted literal into config.nu.
    let env_file = ($env.VP_HOME | path join "env.nu" | to nuon)
    $"\nsource ($env_file)\n" | save --append ($config_dir | path join "config.nu")
    let repaired = (^$nu.current-exe --no-history --execute 'try { let help = (^vp help | complete); if $help.exit_code != 0 { error make {msg: $help.stderr} }; source probe.nu } catch { |err| print -e $err; exit 1 }; exit 0' | complete)
    if $repaired.exit_code != 0 {
      error make {msg: $"Session with the documented source line failed: ($repaired.stdout) ($repaired.stderr)"}
    }
    let repaired_state = ($repaired.stdout | from json)
    if not $repaired_state.loaded or $repaired_state.directory != $session_dir or $repaired_state.data_home != $custom_data {
      error make {msg: "The source workaround did not load Vite+ with the original XDG configuration"}
    }
    print "After adding source to config.nu, a fresh session loaded Vite+: true"
    print "vp help succeeded in the fresh session"
  }
}
