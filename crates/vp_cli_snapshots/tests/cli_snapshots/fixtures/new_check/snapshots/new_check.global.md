# new_check

## `vp create --help`

show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Use any builtin, local or remote template with Vite+.

Arguments:
  [TEMPLATE]             Builtin, local, or remote template name
  [TEMPLATE_OPTIONS]...  Arguments passed to the template without changes

Options:
  --directory <DIR>                      Target directory for the generated project
  --agent <NAME>                         Write coding agent instructions to AGENTS.md, CLAUDE.md, etc.
  --no-agent                             Skip writing coding agent instructions
  --editor <NAME>                        Write editor config files for the specified editor
  --no-editor                            Skip writing editor config files
  --git                                  Initialize a git repository
  --no-git                               Skip git repository initialization
  --hooks                                Set up pre-commit hooks (default in non-interactive mode)
  --no-hooks                             Skip pre-commit hooks setup
  --package-manager <pnpm|npm|yarn|bun>  Use the specified package manager
  --approve-builds                       Approve and run gated dependency build scripts without prompting
  --verbose                              Show detailed scaffolding output
  --interactive                          Enable interactive prompts
  --no-interactive                       Run in non-interactive mode
  --list                                 List all available templates
  -h, --help                             Show this help message

Examples:
  vp create                                      # Interactive mode
  vp create vite                                 # Use create-vite
  vp create vite -- --template react-ts          # Pass template options
  vp create vite:monorepo                        # Create a Vite+ monorepo
  vp create github:user/repo                     # Use a GitHub template
  vp create @your-org                            # Open an org template picker

Documentation: https://viteplus.dev/guide/create
```

## `vp create --list`

list templates

```
VITE+ - The Unified Toolchain for the Web

Usage: vp create --list

List available builtin and popular project templates.

Vite+ Built-in Templates:
  vite:monorepo     Create a new monorepo
  vite:application  Create a new application
  vite:library      Create a new library
  vite:generator    Scaffold a new code generator (monorepo only)

Popular Templates (shorthand):
  vite             Official Vite templates (create-vite)
  @tanstack/start  TanStack applications (@tanstack/cli create)
  next-app         Next.js application (create-next-app)
  nuxt             Nuxt application (create-nuxt)
  react-router     React Router application (create-react-router)
  svelte           Svelte application (sv create)
  vue              Vue application (create-vue)

Examples:
  vp create # interactive mode
  vp create vite # shorthand for create-vite
  vp create @tanstack/start # shorthand for @tanstack/cli create
  vp create <template> -- <options> # pass options to the template

Tip:
  You can use any npm template or git repo with vp create.

Documentation: https://viteplus.dev/guide/create
```

## `vp create --no-interactive`

run in non-interactive mode without template name will show error

**Exit code:** 1

```

A template name is required when running in non-interactive mode

Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Example:
  # Create a new application in non-interactive mode with a custom target directory
  vp create vite:application --no-interactive --directory=apps/my-app

Use `vp create --list` to list all available templates, or run `vp create --help` for more information.
```
