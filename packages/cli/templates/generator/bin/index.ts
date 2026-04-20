#!/usr/bin/env node

import { runTemplateCLI } from 'bingo';

import template from '../src/template.js';

process.exitCode = await runTemplateCLI(template);
