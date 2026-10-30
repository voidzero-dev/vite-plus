# vp pm approve-builds

Approve dependency lifecycle scripts using the selected manager's trust policy. Approve only packages whose install scripts you intend to execute.

```bash
vp pm approve-builds
vp pm approve-builds sharp
vp pm approve-builds '!untrusted-package'
vp pm approve-builds --all
```

## Options

Supply packages to approve, or prefix a name with `!` to request denial where supported. Quote denial arguments to avoid shell history expansion.

| Option  | Description                                         | Notes                                                              |
| ------- | --------------------------------------------------- | ------------------------------------------------------------------ |
| `--all` | Approve all packages pending build-script approval. | Cannot be combined with package names, including names after `--`. |

## Compatibility

- **npm:** 11.16 or later: `npm approve-scripts`. Earlier versions warn and skip. Denials use `npm deny-scripts`.
- **pnpm:** 10.1 or later: `pnpm approve-builds`.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** `bun pm trust` for named packages or `--all`.

| Option  | npm              | pnpm             | Yarn Classic | Yarn Modern | Bun | Notes |
| ------- | ---------------- | ---------------- | ------------ | ----------- | --- | ----- |
| `--all` | 11.16.0 or later | 10.32.0 or later | No           | No          | Yes |       |

With no package names, pnpm opens its interactive flow and npm lists pending packages with `--allow-scripts-pending`. Bun does not open an interactive selector; inspect pending packages with `bun pm untrusted` and then name the packages to trust.

Denial arguments prefixed with `!` require pnpm 11 or later or npm 11.16 or later. npm requires separate approval and denial invocations. Bun skips denial entries with a warning because it uses `trustedDependencies` rather than a denylist.

For Yarn Modern, configure `enableScripts: false` and explicitly allow trusted packages through `dependenciesMeta`.

### npm Build-Script Policy

npm 12 enforces `allowScripts` in `package.json`. Dependency install scripts not covered by the policy are skipped, including implicit native builds. Approval updates policy but does not run scripts skipped earlier. Follow it with [`vp pm rebuild <package>`](./rebuild).

In npm 11.16 through 11.x, this policy is advisory. Scripts still run and npm warns about unreviewed packages.

## Native Options

For native options, see [npm approve-scripts](https://docs.npmjs.com/cli/v11/commands/npm-approve-scripts), [npm deny-scripts](https://docs.npmjs.com/cli/v11/commands/npm-deny-scripts), [pnpm approve-builds](https://pnpm.io/cli/approve-builds), [Bun pm trust](https://bun.com/docs/pm/cli/pm), and [Bun pm untrusted](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).

For script policy configuration, see [npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json), [Yarn enableScripts](https://yarnpkg.com/configuration/yarnrc#enableScripts), and [Bun lifecycle scripts](https://bun.com/docs/pm/lifecycle).
