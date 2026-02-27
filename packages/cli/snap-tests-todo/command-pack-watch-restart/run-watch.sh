#!/bin/bash
# Start vp pack watch in background, redirect output to file, and save PID
vp pack --watch > .watch-output.log 2>&1 &
echo $! > .pid
