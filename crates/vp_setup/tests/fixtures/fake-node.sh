#!/bin/sh
printf '%s\n' "$@" > invocation.txt
printf '%s' "$PATH" > path.txt
printf '%s' "$npm_config_registry" > registry.txt
