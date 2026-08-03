# create_library_in_git_directory

## `git init`


## `vp create vite:library --directory . --no-interactive --no-git --no-hooks --no-agent --no-editor`

create a library in a directory containing only .git

```

Using package name: workspace
◇ Scaffolded . with TypeScript library
• Node <version>  pnpm <version>
→ Next: vp run
```

## `vpt stat-file .git --assert dir`

existing git metadata is preserved

```
.git: dir
```

## `vpt stat-file package.json --assert file`

library template was created

```
package.json: file
```
