#!/bin/bash
# Start vite lib watch in background, redirect output to file, and save PID
vite lib --watch > .watch-output.log 2>&1 &
echo $! > .pid
