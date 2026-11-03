# command_pm_approve_builds_npm12

## `vp pm approve-builds`

no args -> npm approve-scripts --allow-scripts-pending (lists pending)

```
No packages with unreviewed install scripts.
```

## `vp pm approve-builds esbuild`

-> npm approve-scripts esbuild (npm 12 enforces allowScripts, so vp points at vp pm rebuild)

**Exit code:** 1

```
note: npm records the approval in the `allowScripts` field of package.json but does not run scripts a previous install skipped. Run `vp pm rebuild <package>` to execute them.
npm error code ENOMATCH
npm error No installed packages match: esbuild
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
```

## `vp pm approve-builds !core-js`

deny-only -> npm deny-scripts core-js (denial keeps the enforced default, no note)

**Exit code:** 1

```
npm error code ENOMATCH
npm error No installed packages match: core-js
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
```

## `vp pm approve-builds esbuild !core-js`

mixed approve+deny -> rejected, exit non-zero

**Exit code:** 1

```
npm manages approvals and denials separately. Run them as two invocations, e.g. `vp pm approve-builds <approve-pkg>...` then `vp pm approve-builds !<deny-pkg>...`.
```

## `vp pm approve-builds --all`

-> npm approve-scripts --all (rebuild note)

```
note: npm records the approval in the `allowScripts` field of package.json but does not run scripts a previous install skipped. Run `vp pm rebuild <package>` to execute them.
No packages with unreviewed install scripts.
```
