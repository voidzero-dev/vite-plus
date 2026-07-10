# create_org_bundled_dotfiles

## `vp create @your-org:demo --no-interactive --directory my-demo-app`

bundled template with _gitignore/_npmrc

```
◇ Scaffolded my-demo-app
• Node <version>  pnpm <version>
→ Next: cd my-demo-app && vp run
```

## `vpt list-dir my-demo-app --all`

verify _gitignore/_npmrc were renamed and no underscore variants remain

```
.gitignore
.npmrc
.vite-hooks
AGENTS.md
package.json
pnpm-workspace.yaml
src
vite.config.ts
```

## `vpt print-file my-demo-app/.gitignore`

verify _gitignore content was preserved

```
node_modules
dist
```

## `vpt print-file my-demo-app/.npmrc`

verify _npmrc content was preserved

```
auto-install-peers=true
```
