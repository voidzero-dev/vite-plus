#!/bin/sh
# Fake bundled corepack: echoes its invocation with the test root normalized
# for stable snapshots, and simulates corepack clobbering the npm shim on
# `enable` so the test can assert that Vite+ restores it.
# The script lives at <install>/js_runtime/node/<version>/bin/corepack, so the
# install's shim dir is three levels up plus `bin`.
if [ "$1" = "enable" ]; then
  rm -f "$(dirname "$0")/../../../bin/npm"
fi
out="corepack"
for arg in "$@"; do
  out="$out $(printf '%s' "$arg" | sed "s#$PWD#<root>#g")"
done
printf '%s\n' "$out"
