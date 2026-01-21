#!/usr/bin/env bash

# Benchmark script to compare performance with and without auto-install
# Our implementation uses VITE_DISABLE_AUTO_INSTALL=1 to disable nested auto-install execution

AUTO_INSTALL_CMD="node ./packages/cli/src/bin.ts lint"

echo "=== Performance comparison: auto-install enabled vs disabled ==="
echo ""

# Test with auto-install enabled (default behavior)
echo "Testing with auto-install enabled…"
time ${AUTO_INSTALL_CMD}

echo ""

# Test with auto-install disabled (simulating nested execution)
echo "Testing with auto-install disabled (nested execution simulation)…"
time VITE_DISABLE_AUTO_INSTALL=1 ${AUTO_INSTALL_CMD}

echo ""
echo "=== Running detailed benchmark with hyperfine ==="

# Run detailed benchmark comparison
hyperfine -w 2 -r 5 -i \
  -n "auto-install-enabled" "${AUTO_INSTALL_CMD}" \
  -n "auto-install-disabled" "VITE_DISABLE_AUTO_INSTALL=1 ${AUTO_INSTALL_CMD}"
