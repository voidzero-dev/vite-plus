let result = (^./external/vp | complete)
if $result.exit_code != 0 or ($result.stderr | str contains "Could not configure shell profiles") {
  error make {msg: $"Standalone setup failed: ($result.stdout) ($result.stderr)"}
}
{
  directory: ($nu.vendor-autoload-dirs | last)
  data_home: $env.XDG_DATA_HOME
} | to json --raw | print
