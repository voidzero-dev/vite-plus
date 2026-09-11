source env.fish
or begin
    echo "failed to source env.fish"
    exit 1
end

if test "$VP_HOME" != "$EXPECTED_VP_HOME"
    echo "VP_HOME mismatch: expected $EXPECTED_VP_HOME, got $VP_HOME"
    exit 1
end

set -l expected_bin "$EXPECTED_VP_HOME/bin"
if test "$PATH[1]" != "$expected_bin"
    echo "PATH mismatch: expected first entry $expected_bin, got $PATH[1]"
    exit 1
end

set -l bin_count 0
for entry in $PATH
    if test "$entry" = "$expected_bin"
        set bin_count (math $bin_count + 1)
    end
end
if test $bin_count -ne 1
    echo "PATH contains the Vite+ bin directory $bin_count times"
    exit 1
end

functions -q vp
or begin
    echo "env.fish did not define the vp wrapper"
    exit 1
end

set -l vp_output (vp --version)
or begin
    echo "vp --version failed through the Fish wrapper"
    exit 1
end
if test (count $vp_output) -eq 0
    echo "vp --version returned no output"
    exit 1
end

echo "Fish environment setup checks passed"
