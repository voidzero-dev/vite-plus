# Nushell 0.116 shows no suggestions when a parameter completer fails. Without fish, the
# completers must return null so that file completion answers instead (#3272).
let lines = ["vp ", "vpr "]
let path = $env.PATH | where { not ($in | path join fish | path exists) }
let script = $"source env.nu
($lines | to nuon) | each {|line| $line | commandline complete } | to json --raw"
let result = with-env { PATH: $path } { ^$nu.current-exe -n -c $script | complete }
if $result.exit_code != 0 {
  error make { msg: $"completion failed: ($result.stderr)" }
}

let completions = $result.stdout | from json
for index in [0 1] {
  let values = $completions | get $index
  if "env.nu" not-in $values {
    error make { msg: $"`($lines | get $index)` did not fall back to file completion without fish, got: ($values)" }
  }
}

print "Nushell completer checks without fish passed"
