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

## Step 2: Check for Subdirectory

Fetch the repository's root to check if the main package.json is in a subdirectory (like `web/`, `app/`, `frontend/`).

Ask the user:

- Which directory contains the main package.json? (root or subdirectory)
- What Node.js version to use? (22 or 24)
- Which commands to run? (e.g., lint, build, test, type-check)

## Step 3: Update Files

1. **Add to `ecosystem-ci/repo.json`**:

   ```json
   {
     "project-name": {
       "repository": "https://github.com/owner/repo.git",
       "branch": "main",
       "hash": "full-commit-sha",
       "directory": "web" // only if subdirectory is needed
     }
   }
   ```

2. **Add to `.github/workflows/e2e-test.yml`** matrix:
   ```yaml
   - name: project-name
     node-version: 24
     directory: web # only if subdirectory is needed
     command: |
       vite run lint
       vite run build
   ```

## Step 4: Verify

Test the clone locally:

```bash
node ecosystem-ci/clone.ts project-name
```

## Important Notes

- The `directory` field is optional - only add it if the package.json is not in the project root
- If `directory` is specified in repo.json, it must also be specified in the workflow matrix
- `patch-project.ts` automatically handles running `vite migrate` in the correct directory
