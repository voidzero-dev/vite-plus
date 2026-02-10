#!/bin/bash
# Start vite pack watch in background, redirect output to file, and save PID
vite pack --watch > .watch-output.log 2>&1 &
echo $! > .pid
