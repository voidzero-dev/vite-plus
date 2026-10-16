# create_force_keeps_existing_files

## `cd my-app && vp create vite:application --no-interactive --directory .`

a non-empty directory is refused by default

**Exit code:** 1

```
● Using package name: my-app
● Use --directory to specify a different location, remove the directory first, or pass --force to keep the existing files
Target directory "<workspace>/my-app" is not empty
```

## `cd my-app && vp create vite:application --no-interactive --directory . --force`

--force scaffolds around the existing files

```
● Using package name: my-app
● Skipped template files that already exist: .gitignore
◇ Scaffolded . with Vite application
• Node <version>  pnpm <version>
→ Next: vp run
```

## `vpt print-file my-app/README.md`

the existing README is kept and not reformatted

```
#   Mine

*  kept as written
```

## `vpt print-file my-app/notes/todo.md`

files in existing directories are not reformatted

```
*  also kept
```

## `vpt print-file my-app/.gitignore`

the existing .gitignore is kept

```
mine
```

## `vpt stat-file my-app/package.json --assert file`

the template is scaffolded

```
my-app/package.json: file
```

## `cd has-package && vp create vite:application --no-interactive --directory . --force`

a directory that already has a package.json is refused

**Exit code:** 1

```
● Using package name: has-package
Target directory "<workspace>/has-package" already contains a package.json, so --force cannot scaffold a project into it
```
