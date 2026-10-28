# User commands that shadow the built-ins env.nu uses (#3092). Nushell resolves
# names at parse time, so env.nu must call the built-ins with the `%` sigil.
# This script itself uses only keywords and commands that are not shadowed.
def update [] { "user update" }
def length [] { "user length" }
def skip [] { }
def lines [] { }
def parse [] { }
def get [] { }
def default [] { }
def insert [] { }
def reduce [] { }
def is-not-empty [] { }
def load-env [] { }
def hide-env [] { }
def with-env [] { }
def prepend [] { }
def append [] { }
def "str starts-with" [] { }
def "str replace" [] { }
def "path expand" [] { }
def "path exists" [] { }
def "from tsv" [] { }
def rename [] { }
def any [] { }
def fish [] { }

source env.nu

vp env use 20.20.2 --no-install
if ("VP_NODE_VERSION" not-in $env) {
  error make { msg: "vp env use did not set VP_NODE_VERSION" }
}
if $env.VP_NODE_VERSION != "20.20.2" {
  error make { msg: $"VP_NODE_VERSION mismatch: expected 20.20.2, got ($env.VP_NODE_VERSION)" }
}

vp env use --unset
if ("VP_NODE_VERSION" in $env) {
  error make { msg: "vp env use --unset did not remove VP_NODE_VERSION" }
}

if (update) != "user update" {
  error make { msg: "env.nu replaced the user's update command" }
}

print "env.nu loads and works with shadowed built-ins"
