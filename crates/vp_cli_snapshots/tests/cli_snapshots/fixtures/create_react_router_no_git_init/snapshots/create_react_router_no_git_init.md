# create_react_router_no_git_init

Use the real pinned create-react-router CLI with a local template so no GitHub template download is needed. Git identity is disabled only for this case: the generator's initial commit would fail without --no-git-init.

## `vp create react-router@8.5.1 --no-interactive --no-agent --no-editor --no-hooks --git -- app --template ./template --yes --no-install --no-agent-skills --no-motion`

Vite+ passes --no-git-init to the generator and completes integration

```
● pnpm@10.18.0 installing...
● pnpm@10.18.0 installed
◇ Generating project…
● Running: pnpm dlx create-react-router@8.5.1 app --template ./template --yes --no-install --no-agent-skills --no-motion --no-git-init

         create-react-router <version>
      ◼  Directory: Using app as project directory

      ◼  Template: Using ./template...
      ✔  Template copied
      ◼  Skipping agent skill. You can add it later from https://github.com/remix-run/react-router/tree/main/.agents/skills/react-router.
      ◼  Skipping install step. Remember to install dependencies after setup with pnpm install.

  done   That's it!

         Enter your project directory using cd ./app
         Check out README.md for development and deploy instructions.

         Join the community at https://remix.run/discord

● Installing dependencies...
● Dependencies installed
● Formatting code...
● Code formatted
◇ Scaffolded app with ./template + JavaScript
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Git (optional): git -C app add -A && git -C app commit -m "chore: initial commit"
→ Next: cd app && vp run
```

## `node assert-project.cjs`

```
Vite+ integrated and initialized Git without an initial commit.
```
