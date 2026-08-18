source env.fish
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

vp env use 20.18.0 --no-install
or begin
    echo "vp env use failed through the Fish wrapper"
    exit 1
end
if not set -q VP_NODE_VERSION
    echo "vp env use did not set VP_NODE_VERSION"
    exit 1
end
if test "$VP_NODE_VERSION" != "20.18.0"
    echo "VP_NODE_VERSION mismatch: expected 20.18.0, got $VP_NODE_VERSION"
    exit 1
end

vp env use --unset
or begin
    echo "vp env use --unset failed through the Fish wrapper"
    exit 1
end
if set -q VP_NODE_VERSION
    echo "vp env use --unset did not remove VP_NODE_VERSION"
    exit 1
end

echo "Fish environment use checks passed"
