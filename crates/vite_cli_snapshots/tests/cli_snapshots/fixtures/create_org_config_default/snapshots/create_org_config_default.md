# create_org_config_default

## `vp create --no-interactive`

bare vp create picks up create.defaultTemplate from vite.config.ts

**Exit code:** 1

```

A template name is required when running `vp create @your-org` in non-interactive mode.

Available templates in @your-org/create:

  NAME  DESCRIPTION       TEMPLATE
  web   Web app template  @your-org/template-web

Examples:
  # Scaffold a specific template from the org
  vp create @your-org:web --no-interactive

  # Or use a Vite+ built-in template
  vp create vite:application --no-interactive
```
