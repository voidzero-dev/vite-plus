#!/usr/bin/env node

import { runTemplateCLI } from 'bingo';

import template from '../src/template.ts';

process.exitCode = await runTemplateCLI(template);
