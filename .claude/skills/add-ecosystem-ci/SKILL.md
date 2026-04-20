---
name: add-ecosystem-ci
description: Add a new ecosystem-ci test case for testing real-world projects against vite-plus
allowed-tools: Bash, Read, Edit, Write, WebFetch, AskUserQuestion
---

# Add Ecosystem-CI Test Case

Add a new ecosystem-ci test case following this process:

## Step 1: Get Repository Information

Ask the user for the GitHub repository URL if not provided as argument: $ARGUMENTS

Use GitHub CLI to get repository info:

```bash
gh api repos/OWNER/REPO --jq '.default_branch'
gh api repos/OWNER/REPO/commits/BRANCH --jq '.sha'
```

## Step 2: Auto-detect Project Configuration

### 2.1 Check for Subdirectory

Fetch the repository's root to check if the main package.json is in a subdirectory (like `web/`, `app/`, `frontend/`).

### 2.2 Check if Project Already Uses Vite-Plus

Check the project's root `package.json` for `vite-plus` in `dependencies` or `devDependencies`. If the project already uses vite-plus, set `forceFreshMigration: true` in `repo.json`. This tells `patch-project.ts` to set `VITE_PLUS_FORCE_MIGRATE=1` so `vp migrate` forces full dependency rewriting instead of skipping with "already using Vite+".

### 2.3 Auto-detect Commands from GitHub Workflows

Fetch the project's GitHub workflow files to detect available commands:

```bash
# List workflow files
gh api repos/OWNER/REPO/contents/.github/workflows --jq '.[].name'

# Fetch workflow content (for each .yml/.yaml file)
gh api repos/OWNER/REPO/contents/.github/workflows/ci.yml --jq '.content' | base64 -d
```

Look for common patterns in workflow files:

- `pnpm run <command>` / `npm run <command>` / `yarn <command>`
- Commands like: `lint`, `build`, `test`, `type-check`, `typecheck`, `format`, `format:check`
- Map detected commands to `vp` equivalents: `vp run lint`, `vp run build`, etc.

### 2.4 Ask User to Confirm

Present the auto-detected configuration and ask user to confirm or modify:

- Which directory contains the main package.json? (auto-detected or manual)
- What Node.js version to use? (22 or 24, try to detect from workflow)
- Which commands to run? (show detected commands as multi-select options)
- Which OS to run on? (both, ubuntu-only, windows-only) - default: both

## Step 3: Update Files

1. **Add to `ecosystem-ci/repo.json`**:

   ```json
   {
     "project-name": {
       "repository": "https://github.com/owner/repo.git",
       "branch": "main",
       "hash": "full-commit-sha",
       "directory": "web", // only if subdirectory is needed
       "forceFreshMigration": true // only if project already uses vite-plus
     }
   }
   ```

2. **Add to `.github/workflows/e2e-test.yml`** matrix:
   ```yaml
   - name: project-name
     node-version: 24
     directory: web # only if subdirectory is needed
     command: |
       vp run lint
       vp run build
   ```

## Step 4: Verify

### 4.1 Build fresh tgz packages

Always rebuild tgz packages from latest source to avoid using stale cached versions:

```bash
# Rebuild the global CLI first (includes Rust binary + NAPI binding)
pnpm bootstrap-cli

# Pack fresh tgz files into tmp/tgz/
rm -rf tmp/tgz && mkdir -p tmp/tgz
cd packages/core && pnpm pack --pack-destination ../../tmp/tgz && cd ../..
cd packages/test && pnpm pack --pack-destination ../../tmp/tgz && cd ../..
cd packages/cli && pnpm pack --pack-destination ../../tmp/tgz && cd ../..
ls -la tmp/tgz
```

### 4.2 Clone and test locally

```bash
node ecosystem-ci/clone.ts project-name
```

### 4.3 Patch and run commands

```bash
# Run from the ecosystem-ci temp directory
cd $(node -e "const os=require('os'); console.log(os.tmpdir() + '/vite-plus-ecosystem-ci')")

# Migrate the project (uses tgz files from tmp/tgz/)
node /path/to/vite-plus/ecosystem-ci/patch-project.ts project-name

# Run the configured commands
cd project-name
vp run build
```

3. **Add OS exclusion to `.github/workflows/e2e-test.yml`** (if not running on both):

   For ubuntu-only:

   ```yaml
   exclude:
     - os: windows-latest
       project:
         name: project-name
   ```

   For windows-only:

   ```yaml
   exclude:
     - os: ubuntu-latest
       project:
         name: project-name
   ```

## Important Notes

- The `directory` field is optional - only add it if the package.json is not in the project root
- If `directory` is specified in repo.json, it must also be specified in the workflow matrix
- `patch-project.ts` automatically handles running `vp migrate` in the correct directory
- `forceFreshMigration` is required for projects that already have `vite-plus` in their package.json — it sets `VITE_PLUS_FORCE_MIGRATE=1` so `vp migrate` forces full dependency rewriting instead of skipping
- OS exclusions are added to the existing `exclude` section in the workflow matrix
