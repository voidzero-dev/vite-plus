#!/bin/sh
# A system node: `--version` prints the node-version file next to the link.
# Other runs record the marker, the arguments, and $PATH in the working directory.
if [ "$1" = "--version" ]; then
  cat "$(dirname "$0")/node-version"
  exit 0
fi
echo SYSTEM_NODE > invocation.txt
printf '%s\n' "$@" >> invocation.txt
printf '%s' "$PATH" > path.txt
