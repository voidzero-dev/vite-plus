set -eu
. "$VP_HOME/env"
vp env off node >/dev/null
mkdir -p foreign-bin
cat > foreign-bin/node <<'EOF'
#!/bin/sh
# Model a manager finding Vite+ as its fallback while retaining its original PATH.
exec "$VP_HOME/fallback-bin/node" "$@"
EOF
chmod +x foreign-bin/node
PATH="$PWD/foreign-bin:$VP_HOME/fallback-bin:/usr/bin:/bin"
export PATH
test "$(node --version)" = v20.18.0
test "$(VP_PATH_INJECTED_TOOLS=node node --version)" = v20.18.0
# Internal system-first lookup selects the foreign manager, which reaches the managed fallback.
test "$("$VP_HOME/bin/vp" env exec node --version)" = v20.18.0
echo 'Foreign manager fallback terminates with managed Node'
