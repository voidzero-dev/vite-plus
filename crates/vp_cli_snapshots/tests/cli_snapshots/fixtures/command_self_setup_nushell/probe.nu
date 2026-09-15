{
  directory: ($nu.vendor-autoload-dirs | last)
  data_home: $env.XDG_DATA_HOME
  loaded: (($env.VP_HOME | path join "bin") in $env.PATH)
} | to json --raw | print
