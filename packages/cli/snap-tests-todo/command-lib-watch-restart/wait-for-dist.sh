#!/bin/bash
# Wait for dist directory to appear (max 10 seconds)
for i in {1..20}; do
  if [ -d "dist" ]; then
    echo "dist found"
    exit 0
  fi
  sleep 0.5
done
echo "dist not found after 10 seconds"
exit 1
