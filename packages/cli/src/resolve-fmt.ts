/**
 * Oxfmt tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the oxfmt binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes oxfmt for code formatting.
 *
 * Used for: `vite-plus fmt` command
 *
 * Oxfmt is a fast JavaScript/TypeScript formatter written in Rust that
 * provides high-performance code formatting capabilities.
 */

import { dirname, join } from 'node:path';

import { resolve } from './utils/constants.ts';
import { createToolResolution, type ToolResolution } from './utils/tool-resolution.ts';

/**
 * Resolves the oxfmt binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the oxfmt binary
 *   - envs: Environment variables to set when executing oxfmt
 *
 * The environment variables provide runtime context to oxfmt,
 * including Node.js version information and package manager details.
 */
export async function fmt(): Promise<ToolResolution> {
  // Resolve the oxfmt package path first, then navigate to the bin file.
  // The bin/oxfmt subpath is not exported in package.json exports, so we
  // resolve the main entry point and derive the bin path from it.
  // resolve('oxfmt') returns .../oxfmt/dist/index.js, so we need to go up
  // two directories (past 'dist') to reach the package root.
  const oxfmtMainPath = resolve('oxfmt');
  const binPath = join(dirname(dirname(oxfmtMainPath)), 'bin', 'oxfmt');

  return createToolResolution(binPath);
}
