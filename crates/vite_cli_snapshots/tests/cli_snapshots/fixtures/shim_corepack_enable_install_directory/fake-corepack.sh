#!/bin/sh
# Fake bundled corepack: echoes its invocation with the test root normalized
# for stable snapshots, and simulates corepack clobbering the npm shim on
# `enable` so the test can assert that Vite+ restores it.
if [ "$1" = "enable" ]; then
  rm -f "$VP_HOME/bin/npm"
fi
out="corepack"
for arg in "$@"; do
  out="$out $(printf '%s' "$arg" | sed "s#$PWD#<root>#g")"
done
printf '%s\n' "$out"
