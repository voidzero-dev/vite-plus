#!/bin/bash
# Kill children processes first, then the parent
if [ -f .pid ]; then
  PID=$(cat .pid)
  # Kill all child processes of the parent PID
  pkill -P $PID 2>/dev/null || true
  # Then kill the parent process
  kill $PID 2>/dev/null || true
fi
