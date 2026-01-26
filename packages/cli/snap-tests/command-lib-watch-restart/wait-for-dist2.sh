#!/bin/bash
# Wait for dist2 directory to appear (max 24 seconds)
for i in {1..48}; do
  if [ -d "dist2" ]; then
    echo "dist2 found"
    exit 0
  fi
  sleep 0.5
done
echo "dist2 not found after 24 seconds"
echo "=== .watch-output.log ==="
cat .watch-output.log 2>/dev/null || echo "(no log file)"
exit 1
