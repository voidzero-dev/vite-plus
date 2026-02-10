#!/bin/bash
# Wait for dist2 directory to appear (max 40 seconds)
# Rolldown notify polling interval default is 30s
# https://github.com/rolldown/rolldown/blob/097316fb273a697d59b5b6f40d0cb30f30eb4296/packages/rolldown/src/options/input-options.ts#L78
for i in {1..80}; do
  if [ -d "dist2" ]; then
    echo "dist2 found"
    exit 0
  fi
  sleep 0.5
done
echo "dist2 not found after 40 seconds"
echo "=== .watch-output.log ==="
cat .watch-output.log 2>/dev/null || echo "(no log file)"
exit 1
