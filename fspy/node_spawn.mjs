#!/usr/bin/env node

import cp from 'node:child_process'
import fs from 'node:fs'
cp.execFileSync('/home/vscode/esbuild', ['a.js'], { stdio: 'inherit'});
fs.readdirSync('/workspaces');
