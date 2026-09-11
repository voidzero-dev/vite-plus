#!/usr/bin/env node

import { runTemplateCLI, type Template } from 'bingo';

import template from '../src/template.ts';

// runTemplateCLI accepts the base `Template` type, which is wider than the
// strongly typed template returned by createTemplate(). Cast through `unknown`
// to bridge the two.
process.exitCode = await runTemplateCLI(template as unknown as Template);
