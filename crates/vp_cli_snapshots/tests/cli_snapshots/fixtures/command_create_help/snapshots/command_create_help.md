# command_create_help

## `vp create -h`

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

## `vp create --help`

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

## `vp help create`

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
