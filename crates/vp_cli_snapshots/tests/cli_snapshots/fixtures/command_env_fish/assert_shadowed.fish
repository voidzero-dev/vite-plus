# User functions that shadow the built-ins env.fish uses (#3128). fish runs a function before
# the built-in with the same name, so env.fish must call the built-ins with `builtin`.
# This script calls the shadowed names only with `builtin`.
function count
    echo 0
end
function math
    echo 99
end
# Always false: an always-true `contains` would make the PATH cleanup loop in env.fish endless.
function contains
    return 1
end
function complete
end
function commandline
end
function source
end

builtin source env.fish
or begin
    echo "failed to source env.fish"
    exit 1
end

set -gx VP_NODE_VERSION 18.20.0
vp env use --help >/dev/null
or begin
    echo "vp env use --help failed through the Fish wrapper"
    exit 1
end
if test "$VP_NODE_VERSION" != "18.20.0"
    echo "vp env use --help changed VP_NODE_VERSION"
    exit 1
end

vp env use 20.20.2 --no-install
if test "$VP_NODE_VERSION" != "20.20.2"
    echo "vp env use did not set VP_NODE_VERSION, got '$VP_NODE_VERSION'"
    exit 1
end

vp -C . env use 22.23.4 --no-install
if test "$VP_NODE_VERSION" != "22.23.4"
    echo "vp -C . env use did not set VP_NODE_VERSION, got '$VP_NODE_VERSION'"
    exit 1
end

# The vp completion is out of scope: clap_complete registers it with bare `complete` and
# `commandline`, so the user functions above replace it.
set -l candidates (builtin complete -C "vpr b")
if not builtin contains -- build $candidates
    echo "vpr completion did not suggest build, got: $candidates"
    exit 1
end

if test (count a b c) != 0
    echo "env.fish replaced the user's count function"
    exit 1
end

echo "env.fish loads and works with shadowed built-ins"
